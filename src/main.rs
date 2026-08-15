//! `abacus run` — single-pass dispatch: read the ready backlog, open a lane,
//! start a codex worker in it, send the dispatch prompt, wait for settle.
//! Everything stateful is shelled out to `br` and `herdr`; records,
//! acceptance, and evidence chains are deliberately absent (SHIFT-REPORT
//! 2026-08-13 §3).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::time::Instant;

use abacus::{
    BeadOutcome, dispatch_prompt, format_lane_duration, is_agent_prompt_stalled,
    is_dirty_worktree_remove_error, parse_bead_outcome, parse_ready, parse_worktree_created,
    sanitize_agent_name, select_bead, should_reap_lane, version_string,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version") | Some("-V") => {
            println!("abacus {}", version_string());
        }
        Some("--help") | Some("-h") => {
            println!("{}", usage());
        }
        Some("run") => {
            let repo = args.get(1).map(PathBuf::from).unwrap_or_else(|| ".".into());
            if let Err(e) = cmd_run(&repo) {
                eprintln!("abacus run: {e}");
                exit(1);
            }
        }
        Some("merge-jsonl") => {
            let [_, ours, base, theirs] = args.as_slice() else {
                print_usage();
                exit(2);
            };
            if let Err(e) = cmd_merge_jsonl(Path::new(ours), Path::new(base), Path::new(theirs)) {
                eprintln!("abacus merge-jsonl: {e}");
                exit(1);
            }
        }
        _ => {
            print_usage();
            exit(2);
        }
    }
}

fn usage() -> &'static str {
    "usage: abacus run [repo-path]\n       abacus merge-jsonl <ours> <base> <theirs>"
}

fn print_usage() {
    eprintln!("{}", usage());
}

#[derive(serde::Deserialize)]
struct MergeIssue {
    id: String,
    updated_at: String,
}

struct MergeLine<'a> {
    updated_at: String,
    line: &'a str,
}

/// Merge the three snapshots as issue records rather than text lines.
///
/// Inputs are considered in ours/theirs/base order so an exact timestamp tie
/// keeps ours. `BTreeMap` makes the resulting tracker stable by issue id.
fn merge_jsonl<'a>(ours: &'a str, base: &'a str, theirs: &'a str) -> Result<String, String> {
    let mut merged: BTreeMap<String, MergeLine<'a>> = BTreeMap::new();
    for (source, input) in [("ours", ours), ("theirs", theirs), ("base", base)] {
        for (line_index, line) in input.lines().enumerate() {
            let issue: MergeIssue = serde_json::from_str(line).map_err(|e| {
                format!(
                    "cannot parse {source} line {} as an issue: {e}",
                    line_index + 1
                )
            })?;
            if issue.id.is_empty() {
                return Err(format!(
                    "cannot parse {source} line {} as an issue: id is empty",
                    line_index + 1
                ));
            }
            if issue.updated_at.is_empty() {
                return Err(format!(
                    "cannot parse {source} line {} as an issue: updated_at is empty",
                    line_index + 1
                ));
            }

            match merged.get(&issue.id) {
                Some(current) if current.updated_at >= issue.updated_at => {}
                _ => {
                    merged.insert(
                        issue.id,
                        MergeLine {
                            updated_at: issue.updated_at,
                            line,
                        },
                    );
                }
            }
        }
    }

    let mut output = String::new();
    for issue in merged.values() {
        output.push_str(issue.line);
        output.push('\n');
    }
    Ok(output)
}

fn cmd_merge_jsonl(ours: &Path, base: &Path, theirs: &Path) -> Result<(), String> {
    let read = |label: &str, path: &Path| {
        std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {label} file {}: {e}", path.display()))
    };
    let ours_jsonl = read("ours", ours)?;
    let base_jsonl = read("base", base)?;
    let theirs_jsonl = read("theirs", theirs)?;
    let merged = merge_jsonl(&ours_jsonl, &base_jsonl, &theirs_jsonl)?;

    std::fs::write(ours, merged)
        .map_err(|e| format!("cannot write ours file {}: {e}", ours.display()))
}

fn retry_never_engaged_once<Reprompt, Reprobe>(
    initial_outcome: BeadOutcome,
    reprompt: Reprompt,
    reprobe: Reprobe,
) -> Result<(Option<String>, BeadOutcome), String>
where
    Reprompt: FnOnce() -> Result<String, String>,
    Reprobe: FnOnce() -> Result<BeadOutcome, String>,
{
    if initial_outcome != BeadOutcome::NeverEngaged {
        return Ok((None, initial_outcome));
    }

    let settled = reprompt()?;
    let outcome = reprobe()?;
    Ok((Some(settled), outcome))
}

fn cmd_run(repo: &Path) -> Result<(), String> {
    let repo = repo
        .canonicalize()
        .map_err(|e| format!("cannot resolve repo path {}: {e}", repo.display()))?;
    let repo_str = repo.to_string_lossy().into_owned();

    let ready = capture("br", &["ready", "--json"], Some(&repo))?;
    let beads = parse_ready(&ready)?;
    let Some(bead) = select_bead(&beads) else {
        println!("no ready beads in {repo_str}; nothing to dispatch");
        return Ok(());
    };
    capture("br", &["update", &bead.id, "--claim"], Some(&repo))?;
    println!("selected {} — {}", bead.id, bead.title);

    let agent_name = sanitize_agent_name(&bead.id);
    let branch = format!("lane/{}", bead.id);
    let lane_started = Instant::now();
    let lane_result = (|| -> Result<(), String> {
        let created = capture(
            "herdr",
            &[
                "worktree",
                "create",
                "--cwd",
                &repo_str,
                "--branch",
                &branch,
                "--label",
                &bead.id,
                "--no-focus",
            ],
            None,
        )?;
        let lane = parse_worktree_created(&created)?;
        println!(
            "lane open: workspace {} pane {} at {}",
            lane.workspace_id, lane.pane_id, lane.checkout_path
        );

        capture(
            "herdr",
            &[
                "agent",
                "start",
                &agent_name,
                "--kind",
                "codex",
                "--pane",
                &lane.pane_id,
            ],
            None,
        )?;
        println!("codex worker started as agent {agent_name}");

        let prompt = dispatch_prompt(&bead.id, &lane.branch);
        println!(
            "dispatched; waiting for the lane to settle (Ctrl-C detaches, the lane keeps running)"
        );
        let prompt_args = ["agent", "prompt", &agent_name, &prompt, "--wait"];
        let settled = match capture("herdr", &prompt_args, None) {
            Ok(settled) => settled,
            Err(error) if is_agent_prompt_stalled(&error) => {
                eprintln!("agent prompt stalled during worker startup; retrying once");
                capture("herdr", &prompt_args, None)?
            }
            Err(error) => return Err(error),
        };
        println!("{}", settled.trim_end());

        let bead_state = capture("br", &["show", &bead.id, "--json"], Some(&repo))?;
        let initial_outcome = parse_bead_outcome(&bead_state)?;
        if initial_outcome == BeadOutcome::NeverEngaged {
            eprintln!("worker never engaged after startup prompt; retrying once");
        }
        let (retry_settled, outcome) = retry_never_engaged_once(
            initial_outcome,
            || capture("herdr", &prompt_args, None),
            || {
                let bead_state = capture("br", &["show", &bead.id, "--json"], Some(&repo))?;
                parse_bead_outcome(&bead_state)
            },
        )?;
        if let Some(retry_settled) = retry_settled {
            println!("{}", retry_settled.trim_end());
        }
        if should_reap_lane(outcome) {
            let remove_args = ["worktree", "remove", "--workspace", &lane.workspace_id];
            match capture("herdr", &remove_args, None) {
                Ok(_) => {}
                Err(error) if is_dirty_worktree_remove_error(&error) => {
                    eprintln!(
                        "WARNING: completed lane left uncommitted changes in workspace {}; \
                         forcing removal. This is a protocol violation worth investigating.",
                        lane.workspace_id
                    );
                    capture(
                        "herdr",
                        &[
                            "worktree",
                            "remove",
                            "--workspace",
                            &lane.workspace_id,
                            "--force",
                        ],
                        None,
                    )?;
                }
                Err(error) => return Err(error),
            }
            println!("lane reaped: workspace {}", lane.workspace_id);
        }

        match outcome {
            BeadOutcome::Completed => {
                let duration = format_lane_duration(lane_started.elapsed().as_secs());
                println!("bead {} is closed; worker completed in {duration}", bead.id);
                Ok(())
            }
            BeadOutcome::Incomplete => Err(format!(
                "bead {} is in_progress; worker engaged but the run is incomplete",
                bead.id
            )),
            BeadOutcome::NeverEngaged => {
                Err(format!("bead {} is open; worker never engaged", bead.id))
            }
        }
    })();

    lane_result.map_err(|error| {
        let duration = format_lane_duration(lane_started.elapsed().as_secs());
        format!("{error} after {duration}")
    })
}

/// Run a command, capture stdout; a non-zero exit becomes an error carrying
/// the command line and stderr, because the substrate CLI's own message is
/// the diagnosis.
fn capture(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("failed to spawn {program}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`{program} {}` failed ({}): {}",
            args.join(" "),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_text_describes_the_run_command() {
        assert!(usage().contains("abacus run"));
    }

    #[test]
    fn never_engaged_retry_runs_one_reprompt_and_one_reprobe_only() {
        let mut prompt_calls = 0;
        let mut probe_calls = 0;

        let (settled, outcome) = retry_never_engaged_once(
            BeadOutcome::NeverEngaged,
            || {
                prompt_calls += 1;
                Ok("second prompt settled".to_owned())
            },
            || {
                probe_calls += 1;
                Ok(BeadOutcome::NeverEngaged)
            },
        )
        .unwrap();

        assert_eq!(settled.as_deref(), Some("second prompt settled"));
        assert_eq!(outcome, BeadOutcome::NeverEngaged);
        assert_eq!(prompt_calls, 1, "only one recovery prompt is allowed");
        assert_eq!(probe_calls, 1, "the recovery prompt gets one re-probe");
    }

    #[test]
    fn merge_jsonl_uses_the_line_with_the_latest_updated_at() {
        let ours = concat!(
            r#"{"id":"ab-a","updated_at":"2026-08-13T10:00:02Z","side":"ours"}"#,
            "\n",
            r#"{"id":"ab-b","updated_at":"2026-08-13T10:00:04Z","side":"ours"}"#,
            "\n",
        );
        let base = concat!(
            r#"{"id":"ab-a","updated_at":"2026-08-13T10:00:01Z","side":"base"}"#,
            "\n",
            r#"{"id":"ab-b","updated_at":"2026-08-13T10:00:01Z","side":"base"}"#,
            "\n",
        );
        let theirs = concat!(
            r#"{"id":"ab-a","updated_at":"2026-08-13T10:00:03Z","side":"theirs"}"#,
            "\n",
            r#"{"id":"ab-b","updated_at":"2026-08-13T10:00:03Z","side":"theirs"}"#,
            "\n",
        );

        let merged = merge_jsonl(ours, base, theirs).unwrap();

        assert_eq!(
            merged,
            concat!(
                r#"{"id":"ab-a","updated_at":"2026-08-13T10:00:03Z","side":"theirs"}"#,
                "\n",
                r#"{"id":"ab-b","updated_at":"2026-08-13T10:00:04Z","side":"ours"}"#,
                "\n",
            )
        );
    }

    #[test]
    fn merge_jsonl_unions_ids_from_all_three_inputs() {
        let ours = concat!(
            r#"{"id":"ab-ours","updated_at":"2026-08-13T10:00:01Z"}"#,
            "\n",
        );
        let base = concat!(
            r#"{"id":"ab-base","updated_at":"2026-08-13T10:00:01Z"}"#,
            "\n",
        );
        let theirs = concat!(
            r#"{"id":"ab-theirs","updated_at":"2026-08-13T10:00:01Z"}"#,
            "\n",
        );

        let merged = merge_jsonl(ours, base, theirs).unwrap();

        assert_eq!(
            merged,
            concat!(
                r#"{"id":"ab-base","updated_at":"2026-08-13T10:00:01Z"}"#,
                "\n",
                r#"{"id":"ab-ours","updated_at":"2026-08-13T10:00:01Z"}"#,
                "\n",
                r#"{"id":"ab-theirs","updated_at":"2026-08-13T10:00:01Z"}"#,
                "\n",
            )
        );
    }

    #[test]
    fn merge_jsonl_rejects_a_malformed_line_in_any_input() {
        let valid = concat!(
            r#"{"id":"ab-valid","updated_at":"2026-08-13T10:00:01Z"}"#,
            "\n",
        );
        let malformed = "{not json}\n";

        assert!(merge_jsonl(malformed, valid, valid).is_err());
        assert!(merge_jsonl(valid, malformed, valid).is_err());
        assert!(merge_jsonl(valid, valid, malformed).is_err());
    }
}

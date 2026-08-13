//! `abacus run` — single-pass dispatch: read the ready backlog, open a lane,
//! start a codex worker in it, send the dispatch prompt, wait for settle.
//! Everything stateful is shelled out to `br` and `herdr`; records,
//! acceptance, and evidence chains are deliberately absent (SHIFT-REPORT
//! 2026-08-13 §3).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

use abacus::{
    BeadOutcome, dispatch_prompt, is_agent_prompt_stalled, parse_bead_outcome, parse_ready,
    parse_worktree_created, select_bead, should_reap_lane, version_string,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version") | Some("-V") => {
            println!("abacus {}", version_string());
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

fn print_usage() {
    eprintln!("usage: abacus run [repo-path]\n       abacus merge-jsonl <ours> <base> <theirs>");
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

    let branch = format!("lane/{}", bead.id);
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
            &bead.id,
            "--kind",
            "codex",
            "--pane",
            &lane.pane_id,
        ],
        None,
    )?;
    println!("codex worker started as agent {}", bead.id);

    let prompt = dispatch_prompt(&bead.id, &lane.branch);
    println!(
        "dispatched; waiting for the lane to settle (Ctrl-C detaches, the lane keeps running)"
    );
    let prompt_args = ["agent", "prompt", &bead.id, &prompt, "--wait"];
    let settled = match capture("herdr", &prompt_args, None) {
        Ok(settled) => settled,
        Err(error) if is_agent_prompt_stalled(&error) => {
            eprintln!("agent prompt stalled during worker startup; retrying once");
            capture("herdr", &prompt_args, None)?
        }
        Err(error) => return Err(error),
    };
    println!("{}", settled.trim_end());

    let bead_state = capture(
        "br",
        &["show", &bead.id, "--json"],
        Some(Path::new(&lane.checkout_path)),
    )?;
    let outcome = parse_bead_outcome(&bead_state)?;
    if should_reap_lane(outcome) {
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
        println!("lane reaped: workspace {}", lane.workspace_id);
    }

    match outcome {
        BeadOutcome::Completed => {
            println!("bead {} is closed; worker completed", bead.id);
            Ok(())
        }
        BeadOutcome::Incomplete => Err(format!(
            "bead {} is in_progress; worker engaged but the run is incomplete",
            bead.id
        )),
        BeadOutcome::NeverEngaged => Err(format!("bead {} is open; worker never engaged", bead.id)),
    }
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

//! Lane lifecycle phases for `abacus run` and `abacus drain`.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::{
    BeadOutcome, Lane, ReadyBead, dispatch_prompt, format_lane_duration, is_agent_prompt_stalled,
    is_dirty_worktree_remove_error, parse_bead_outcome, parse_worktree_created, should_reap_lane,
};

/// Everything needed to repeat a prompt after a worker never engages.
pub struct LanePrompt {
    agent_name: String,
    prompt: String,
}

/// Open the lane worktree and start its Codex worker.
pub fn lane_open(repo_str: &str, bead: &ReadyBead, agent_name: &str) -> Result<Lane, String> {
    let branch = format!("lane/{}", bead.id);
    let created = capture(
        "herdr",
        &[
            "worktree",
            "create",
            "--cwd",
            repo_str,
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
            agent_name,
            "--kind",
            "codex",
            "--pane",
            &lane.pane_id,
        ],
        None,
    )?;
    println!("codex worker started as agent {agent_name}");
    Ok(lane)
}

/// Dispatch the initial worker prompt, retrying the observed startup race once.
pub fn lane_prompt(
    bead: &ReadyBead,
    lane: &Lane,
    default_branch: &str,
    agent_name: &str,
) -> Result<LanePrompt, String> {
    let prompt = dispatch_prompt(&bead.id, &lane.branch, default_branch);
    println!(
        "dispatched; waiting for the lane to settle (Ctrl-C detaches, the lane keeps running)"
    );
    let prompt_args = ["agent", "prompt", agent_name, &prompt, "--wait"];
    let settled = match capture("herdr", &prompt_args, None) {
        Ok(settled) => settled,
        Err(error) if is_agent_prompt_stalled(&error) => {
            eprintln!("agent prompt stalled during worker startup; retrying once");
            capture("herdr", &prompt_args, None)?
        }
        Err(error) => return Err(error),
    };
    println!("{}", settled.trim_end());
    Ok(LanePrompt {
        agent_name: agent_name.to_owned(),
        prompt,
    })
}

/// Probe the worker outcome, repeat a never-engaged prompt once, reap when
/// allowed, and map the settled state to the existing command result.
pub fn lane_settle<Reap>(
    repo: &Path,
    bead: &ReadyBead,
    lane_started: Instant,
    prompt: &LanePrompt,
    reap: Reap,
) -> Result<(), String>
where
    Reap: FnOnce(BeadOutcome) -> Result<(), String>,
{
    let initial_outcome = probe_bead_outcome(repo, &bead.id)?;
    if initial_outcome == BeadOutcome::NeverEngaged {
        eprintln!("worker never engaged after startup prompt; retrying once");
    }
    let prompt_args = [
        "agent",
        "prompt",
        &prompt.agent_name,
        &prompt.prompt,
        "--wait",
    ];
    let (retry_settled, outcome) = retry_never_engaged_once(
        initial_outcome,
        || capture("herdr", &prompt_args, None),
        || probe_bead_outcome(repo, &bead.id),
    )?;
    if let Some(retry_settled) = retry_settled {
        println!("{}", retry_settled.trim_end());
    }

    reap(outcome)?;

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
        BeadOutcome::NeverEngaged => Err(format!("bead {} is open; worker never engaged", bead.id)),
    }
}

/// Reap a completed lane, escalating a dirty-worktree refusal to force.
pub fn lane_reap(outcome: BeadOutcome, lane: &Lane) -> Result<(), String> {
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
    Ok(())
}

pub fn retry_never_engaged_once<Reprompt, Reprobe>(
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

pub fn retry_probe_once<T, Probe, Delay>(mut probe: Probe, delay: Delay) -> Result<T, String>
where
    Probe: FnMut() -> Result<T, String>,
    Delay: FnOnce(),
{
    match probe() {
        Ok(result) => Ok(result),
        Err(_) => {
            delay();
            probe()
        }
    }
}

fn probe_bead_outcome(repo: &Path, bead_id: &str) -> Result<BeadOutcome, String> {
    let bead_state = retry_probe_once(
        || capture("br", &["show", bead_id, "--json"], Some(repo)),
        || {
            eprintln!("bead outcome probe failed; retrying once after 2 seconds");
            std::thread::sleep(Duration::from_secs(2));
        },
    )?;
    parse_bead_outcome(&bead_state)
}

/// Run a command, capture stdout; a non-zero exit becomes an error carrying
/// the command line and stderr, because the substrate CLI's own message is
/// the diagnosis.
pub fn capture(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
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

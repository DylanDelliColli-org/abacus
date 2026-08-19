//! Lane lifecycle phases for `abacus run` and `abacus drain`.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::{
    BeadOutcome, Lane, ReadyBead, dispatch_prompt, format_lane_duration, is_agent_prompt_stalled,
    is_dirty_worktree_remove_error, parse_bead_outcome, parse_worktree_created, should_reap_lane,
};

// ADR 0005 D8 moves every deployed grammar into the review module when it lands.
pub(crate) const BLOCKED_COMMENT_TOKEN: &str = "BLOCKED";

/// The live state of a lane, re-derived from substrate probes each cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LaneState {
    Authoring,
    Blocked,
    AwaitingReview,
    ReworkRequested,
    Merged,
    Stalled,
}

impl LaneState {
    fn report_label(self) -> &'static str {
        match self {
            Self::Authoring => "authoring",
            Self::Blocked => "blocked",
            Self::AwaitingReview => "awaiting-review",
            Self::ReworkRequested => "rework-requested",
            Self::Merged => "merged",
            Self::Stalled => "stalled",
        }
    }
}

/// The part of `gh pr view` needed by lane-state derivation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestProbe {
    pub state: PullRequestState,
    pub head_sha: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
}

/// Review input is intentionally owned by the lane layer so the review
/// cluster can plug its parser into this truth table without duplicating it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjudicationDisposition {
    Accepted,
    Rework,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdjudicationProbe<'a> {
    pub disposition: AdjudicationDisposition,
    pub adjudicated_head: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct LaneStateInputs<'a> {
    pub bead_outcome: BeadOutcome,
    pub worker_active: bool,
    pub pull_request: Option<&'a PullRequestProbe>,
    pub verdict_heading_count: usize,
    pub latest_adjudication: Option<AdjudicationProbe<'a>>,
}

/// Apply ADR 0005 D1's ordering. Merged and Blocked are absorbing for the
/// current cycle; a rework ruling applies only to the exact adjudicated head.
pub fn derive_lane_state(inputs: LaneStateInputs<'_>) -> LaneState {
    if inputs
        .pull_request
        .is_some_and(|pr| pr.state == PullRequestState::Merged)
    {
        return LaneState::Merged;
    }
    if inputs.bead_outcome == BeadOutcome::Blocked {
        return LaneState::Blocked;
    }
    if let (Some(pr), Some(adjudication)) = (inputs.pull_request, inputs.latest_adjudication)
        && pr.state == PullRequestState::Open
        && adjudication.disposition == AdjudicationDisposition::Rework
        && pr.head_sha.as_deref() == Some(adjudication.adjudicated_head)
    {
        return LaneState::ReworkRequested;
    }
    if inputs.bead_outcome == BeadOutcome::Completed
        && inputs
            .pull_request
            .is_some_and(|pr| pr.state == PullRequestState::Open)
    {
        // An accepted but unmerged adjudication remains AwaitingReview.
        // The heading count is a cycle-bookkeeping input used by review
        // launch; it is deliberately not a substitute for PR/comment facts.
        let _ = inputs.verdict_heading_count;
        return LaneState::AwaitingReview;
    }
    if inputs.bead_outcome == BeadOutcome::Incomplete && inputs.worker_active {
        LaneState::Authoring
    } else {
        LaneState::Stalled
    }
}

#[derive(Debug, Default)]
pub struct MorningReport {
    lanes: std::collections::BTreeMap<&'static str, Vec<LaneReportEntry>>,
}

#[derive(Debug)]
struct LaneReportEntry {
    bead_id: String,
    elapsed_secs: u64,
}

impl MorningReport {
    pub fn record_state(&mut self, state: LaneState, bead_id: &str, elapsed_secs: u64) {
        self.record(state.report_label(), bead_id, elapsed_secs);
    }

    pub fn record_completed(&mut self, bead_id: &str, elapsed_secs: u64) {
        self.record("completed", bead_id, elapsed_secs);
    }

    fn record(&mut self, label: &'static str, bead_id: &str, elapsed_secs: u64) {
        self.lanes.entry(label).or_default().push(LaneReportEntry {
            bead_id: bead_id.to_owned(),
            elapsed_secs,
        });
    }

    pub fn render(&self) -> String {
        const ORDER: [&str; 7] = [
            "completed",
            "blocked",
            "awaiting-review",
            "rework-requested",
            "merged",
            "stalled",
            "authoring",
        ];
        let mut lines = Vec::new();
        for label in ORDER {
            let Some(entries) = self.lanes.get(label) else {
                continue;
            };
            let details = entries
                .iter()
                .map(|entry| {
                    format!(
                        "{} {}",
                        entry.bead_id,
                        format_lane_duration(entry.elapsed_secs)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("{label}: {} [{details}]", entries.len()));
        }
        lines.join("\n")
    }
}

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

/// Probe the worker outcome and repeat a never-engaged prompt once.
/// Command-specific classification and reaping happen in the caller.
pub fn lane_settle(
    repo: &Path,
    bead: &ReadyBead,
    prompt: &LanePrompt,
) -> Result<BeadOutcome, String> {
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

    Ok(outcome)
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

/// Try to reap a blocked lane without ever escalating to `--force`.
/// A dirty refusal is the expected parked outcome, not an engine error.
pub fn lane_reap_blocked(lane: &Lane) -> Result<bool, String> {
    let remove_args = ["worktree", "remove", "--workspace", &lane.workspace_id];
    match capture("herdr", &remove_args, None) {
        Ok(_) => {
            println!("lane reaped: workspace {}", lane.workspace_id);
            Ok(true)
        }
        Err(error) if is_dirty_worktree_remove_error(&error) => {
            eprintln!(
                "blocked lane workspace {} is dirty; leaving it standing",
                lane.workspace_id
            );
            Ok(false)
        }
        Err(error) => Err(error),
    }
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

pub fn probe_bead_outcome(repo: &Path, bead_id: &str) -> Result<BeadOutcome, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs<'a>(
        bead_outcome: BeadOutcome,
        worker_active: bool,
        pull_request: Option<&'a PullRequestProbe>,
    ) -> LaneStateInputs<'a> {
        LaneStateInputs {
            bead_outcome,
            worker_active,
            pull_request,
            verdict_heading_count: 0,
            latest_adjudication: None,
        }
    }

    #[test]
    fn lane_state_derivation_truth_table_for_worker_and_pr_probes() {
        let open = PullRequestProbe {
            state: PullRequestState::Open,
            head_sha: Some("head-1".into()),
        };
        let merged = PullRequestProbe {
            state: PullRequestState::Merged,
            head_sha: Some("head-1".into()),
        };

        assert_eq!(
            derive_lane_state(inputs(BeadOutcome::Incomplete, true, None)),
            LaneState::Authoring
        );
        assert_eq!(
            derive_lane_state(inputs(BeadOutcome::Blocked, false, None)),
            LaneState::Blocked
        );
        assert_eq!(
            derive_lane_state(inputs(BeadOutcome::Completed, false, Some(&open))),
            LaneState::AwaitingReview
        );
        assert_eq!(
            derive_lane_state(inputs(BeadOutcome::Completed, false, Some(&merged))),
            LaneState::Merged
        );
        assert_eq!(
            derive_lane_state(inputs(BeadOutcome::Incomplete, false, None)),
            LaneState::Stalled
        );
        assert_eq!(
            derive_lane_state(inputs(BeadOutcome::NeverEngaged, false, None)),
            LaneState::Stalled
        );
    }

    #[test]
    fn lane_state_derivation_uses_adjudicated_head_and_not_pending_status() {
        let mut open = PullRequestProbe {
            state: PullRequestState::Open,
            head_sha: Some("head-1".into()),
        };
        let accepted = AdjudicationProbe {
            disposition: AdjudicationDisposition::Accepted,
            adjudicated_head: "head-1",
        };
        let rework = AdjudicationProbe {
            disposition: AdjudicationDisposition::Rework,
            adjudicated_head: "head-1",
        };
        let mut review_inputs = inputs(BeadOutcome::Completed, false, Some(&open));
        review_inputs.verdict_heading_count = 0;
        assert_eq!(
            derive_lane_state(review_inputs),
            LaneState::AwaitingReview,
            "an open PR with no verdict comment is awaiting review; combined status is not an input"
        );

        review_inputs.latest_adjudication = Some(accepted);
        assert_eq!(
            derive_lane_state(review_inputs),
            LaneState::AwaitingReview,
            "accepted remains awaiting merge"
        );

        review_inputs.latest_adjudication = Some(rework);
        assert_eq!(derive_lane_state(review_inputs), LaneState::ReworkRequested);

        open.head_sha = Some("head-2".into());
        let mut changed_head = inputs(BeadOutcome::Completed, false, Some(&open));
        changed_head.latest_adjudication = Some(rework);
        assert_eq!(
            derive_lane_state(changed_head),
            LaneState::AwaitingReview,
            "a new head proves rework happened and needs another review"
        );
    }

    #[test]
    fn morning_report_renders_every_settle_class_with_bead_ids() {
        let mut report = MorningReport::default();
        report.record_completed("ab-done", 7);
        report.record_state(LaneState::Blocked, "ab-blocked", 61);
        report.record_state(LaneState::AwaitingReview, "ab-review", 2);
        report.record_state(LaneState::ReworkRequested, "ab-rework", 3);
        report.record_state(LaneState::Merged, "ab-merged", 4);
        report.record_state(LaneState::Stalled, "ab-stalled", 5);

        let rendered = report.render();
        assert!(rendered.contains("completed: 1 [ab-done 7s]"));
        assert!(rendered.contains("blocked: 1 [ab-blocked 1m01s]"));
        assert!(rendered.contains("awaiting-review: 1 [ab-review 2s]"));
        assert!(rendered.contains("rework-requested: 1 [ab-rework 3s]"));
        assert!(rendered.contains("merged: 1 [ab-merged 4s]"));
        assert!(rendered.contains("stalled: 1 [ab-stalled 5s]"));
        assert!(
            !rendered.contains("authoring:"),
            "empty classes are omitted"
        );
    }
}

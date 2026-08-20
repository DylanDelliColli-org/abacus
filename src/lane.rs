//! Lane lifecycle phases for `abacus run` and `abacus drain`.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use crate::review::{Adjudication, FindingDisposition};
use crate::{
    BeadOutcome, Lane, ReadyBead, dispatch_prompt, format_lane_duration, is_agent_prompt_stalled,
    is_dirty_worktree_remove_error, parse_bead_outcome, parse_worktree_created, should_reap_lane,
    target_rust_version,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneReapPolicy {
    Never,
    CleanOnly,
    ForceAllowed,
}

pub fn lane_reap_policy(state: LaneState) -> LaneReapPolicy {
    match state {
        LaneState::Merged => LaneReapPolicy::ForceAllowed,
        LaneState::Blocked => LaneReapPolicy::CleanOnly,
        LaneState::Authoring
        | LaneState::AwaitingReview
        | LaneState::ReworkRequested
        | LaneState::Stalled => LaneReapPolicy::Never,
    }
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
    if let (Some(pr), Some(adjudication)) = (inputs.pull_request, inputs.latest_adjudication) {
        if pr.state == PullRequestState::Open
            && adjudication.disposition == AdjudicationDisposition::Rework
            && pr.head_sha.as_deref() == Some(adjudication.adjudicated_head)
        {
            return LaneState::ReworkRequested;
        }
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

fn open_lane_worktree(repo_str: &str, bead_id: &str) -> Result<Lane, String> {
    let branch = format!("lane/{bead_id}");
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
            bead_id,
            "--no-focus",
        ],
        None,
    )?;
    let lane = parse_worktree_created(&created)?;
    println!(
        "lane open: workspace {} pane {} at {}",
        lane.workspace_id, lane.pane_id, lane.checkout_path
    );
    Ok(lane)
}

pub fn lane_start_agent(lane: &Lane, agent_name: &str) -> Result<(), String> {
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
    Ok(())
}

/// Open the lane worktree and start its Codex worker.
pub fn lane_open(repo_str: &str, bead: &ReadyBead, agent_name: &str) -> Result<Lane, String> {
    let lane = open_lane_worktree(repo_str, &bead.id)?;
    lane_start_agent(&lane, agent_name)?;
    Ok(lane)
}

/// Recreate a vanished warm lane only when no workspace or checkout survives.
pub fn lane_recover(repo_str: &str, bead_id: &str, agent_name: &str) -> Result<Lane, String> {
    let lane = open_lane_worktree(repo_str, bead_id)?;
    lane_start_agent(&lane, agent_name)?;
    println!("warm lane recovered for {bead_id} on {}", lane.branch);
    Ok(lane)
}

/// Open a surviving Git worktree in Herdr, then restart its deterministic agent.
pub fn lane_open_existing_worktree(
    repo_str: &str,
    bead_id: &str,
    agent_name: &str,
) -> Result<Lane, String> {
    let branch = format!("lane/{bead_id}");
    let opened = capture(
        "herdr",
        &[
            "worktree",
            "open",
            "--cwd",
            repo_str,
            "--branch",
            &branch,
            "--label",
            bead_id,
            "--no-focus",
        ],
        None,
    )?;
    let lane = parse_worktree_created(&opened)?;
    lane_start_agent(&lane, agent_name)?;
    println!(
        "surviving worktree reopened for {bead_id}: workspace {} on {}",
        lane.workspace_id, lane.branch
    );
    Ok(lane)
}

fn current_codex_context_percent(pane: &str) -> Option<u8> {
    pane.lines().rev().find_map(|line| {
        let line = line.trim();
        let model = line.split_whitespace().next()?;
        let model_identity = model
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
            && model
                .chars()
                .any(|character| character.is_ascii_alphabetic())
            && model.chars().any(|character| character.is_ascii_digit());
        if !model_identity {
            return None;
        }

        let (_, context) = line.rsplit_once(" · Context ")?;
        let (percent, suffix) = context.split_once("% used")?;
        if !suffix.is_empty() && !suffix.starts_with(" · ") {
            return None;
        }
        percent.parse().ok()
    })
}

fn should_nudge_after_settle(baseline_context: Option<u8>, pane: &str) -> bool {
    baseline_context.is_none() || current_codex_context_percent(pane) == baseline_context
}

fn pasted_composer_is_visible(pane: &str) -> bool {
    pane.lines().rev().take(12).any(|line| {
        let line = line.trim_start();
        line.starts_with('›') && line.contains("Pasted Content") && line.contains("chars")
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptOutcome {
    Settled(String),
    TrackerObserved {
        settled: String,
        outcome: BeadOutcome,
    },
    NeverEngaged {
        error: String,
    },
}

/// Prompt an agent through the shared startup-race seam.
///
/// Herdr can report a successful settled prompt while a fresh Codex TUI still
/// has produced no worker-authored progress. A zero-context settle receives
/// one Enter regardless of whether Codex has rendered its collapsed pasted
/// composer yet. The composer rendering is logged only as diagnostic evidence;
/// it never gates recovery. Observe the turn start, then wait for its terminal
/// state instead of pasting again.
pub fn prompt_agent(
    agent_name: &str,
    pane_id: &str,
    prompt: &str,
    stall_context: &str,
) -> Result<PromptOutcome, String> {
    let baseline_pane = capture("herdr", &["pane", "read", pane_id, "--lines", "40"], None)?;
    let baseline_context = current_codex_context_percent(&baseline_pane);
    prompt_agent_with_tracker_probe(
        agent_name,
        pane_id,
        prompt,
        stall_context,
        baseline_context,
        || Ok(None),
    )
}

fn prompt_agent_with_tracker_probe<Probe>(
    agent_name: &str,
    pane_id: &str,
    prompt: &str,
    stall_context: &str,
    baseline_context: Option<u8>,
    mut tracker_probe: Probe,
) -> Result<PromptOutcome, String>
where
    Probe: FnMut() -> Result<Option<BeadOutcome>, String>,
{
    let prompt_args = ["agent", "prompt", agent_name, prompt, "--wait"];
    let settled = match capture("herdr", &prompt_args, None) {
        Ok(settled) => settled,
        Err(error) if is_agent_prompt_stalled(&error) => {
            eprintln!("agent prompt stalled during {stall_context}; retrying once");
            capture("herdr", &prompt_args, None)?
        }
        Err(error) => return Err(error),
    };

    let tracker_outcome = tracker_probe()?;
    if let Some(outcome @ (BeadOutcome::Completed | BeadOutcome::Blocked)) = tracker_outcome {
        eprintln!("tracker reported {outcome:?} after settle; skipping pane recovery");
        return Ok(PromptOutcome::TrackerObserved { settled, outcome });
    }

    let pane = capture("herdr", &["pane", "read", pane_id, "--lines", "40"], None)?;
    if !should_nudge_after_settle(baseline_context, &pane) {
        return Ok(match tracker_outcome {
            Some(outcome) => PromptOutcome::TrackerObserved { settled, outcome },
            None => PromptOutcome::Settled(settled),
        });
    }

    let composer_diagnostic = if pasted_composer_is_visible(&pane) {
        "pasted composer visible"
    } else {
        "composer not yet visible or empty"
    };
    let baseline_diagnostic = baseline_context
        .map(|percent| format!("context unchanged at {percent}%"))
        .unwrap_or_else(|| "context baseline unavailable".to_owned());
    eprintln!(
        "agent prompt had a zero-effect settle ({baseline_diagnostic}; {composer_diagnostic}); \
         nudging Enter once"
    );
    capture("herdr", &["agent", "send-keys", agent_name, "Enter"], None)?;
    if let Err(error) = capture(
        "herdr",
        &[
            "agent",
            "wait",
            agent_name,
            "--until",
            "working",
            "--timeout",
            "5000",
        ],
        None,
    ) {
        eprintln!("Enter nudge produced no observed worker turn: {error}");
        return Ok(PromptOutcome::NeverEngaged { error });
    }
    let settled = capture(
        "herdr",
        &[
            "agent", "wait", agent_name, "--until", "done", "--until", "blocked",
        ],
        None,
    )?;
    if let Some(outcome) = tracker_probe()? {
        return Ok(PromptOutcome::TrackerObserved { settled, outcome });
    }
    Ok(PromptOutcome::Settled(settled))
}

fn tracker_outcome(repo: &Path, bead_id: &str) -> Result<Option<BeadOutcome>, String> {
    probe_bead_outcome(repo, bead_id).map(Some)
}

/// Dispatch the initial worker prompt, retrying the observed startup race once.
pub fn lane_prompt(
    repo: &Path,
    bead: &ReadyBead,
    lane: &Lane,
    default_branch: &str,
    agent_name: &str,
) -> Result<PromptOutcome, String> {
    let rust_version = target_rust_version(Path::new(&lane.checkout_path))?;
    let prompt = dispatch_prompt(
        &bead.id,
        &lane.branch,
        default_branch,
        rust_version.as_deref(),
    );
    println!(
        "dispatched; waiting for the lane to settle (Ctrl-C detaches, the lane keeps running)"
    );
    let baseline_pane = capture(
        "herdr",
        &["pane", "read", &lane.pane_id, "--lines", "40"],
        None,
    )?;
    let baseline_context = current_codex_context_percent(&baseline_pane);
    let outcome = prompt_agent_with_tracker_probe(
        agent_name,
        &lane.pane_id,
        &prompt,
        "worker startup",
        baseline_context,
        || tracker_outcome(repo, &bead.id),
    )?;
    if let PromptOutcome::Settled(settled) | PromptOutcome::TrackerObserved { settled, .. } =
        &outcome
    {
        println!("{}", settled.trim_end());
    }
    Ok(outcome)
}

pub fn rework_prompt(bead_id: &str, branch: &str, adjudication: &Adjudication) -> String {
    let accepted_findings = adjudication
        .findings
        .iter()
        .filter(|finding| finding.disposition == FindingDisposition::Accepted)
        .map(|finding| {
            format!(
                "- Finding {} destination {}: {}",
                finding.finding_number, finding.context, finding.prose
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Resume work on bead {bead_id} in the existing warm lane.\n\
         Stay on branch {branch}; update its existing pull request in place. Do not create a new \
         branch, worktree, or pull request.\n\
         The adjudicated head is {}.\n\
         Apply the accepted findings below, including each stated destination, then run the bead's \
         required verification, commit, push this same branch, and return the PR to review.\n\
         {accepted_findings}",
        adjudication.adjudicated_head
    )
}

pub fn lane_prompt_rework(
    repo: &Path,
    bead_id: &str,
    lane: &Lane,
    agent_name: &str,
    adjudication: &Adjudication,
) -> Result<PromptOutcome, String> {
    let prompt = rework_prompt(bead_id, &lane.branch, adjudication);
    let baseline_pane = capture(
        "herdr",
        &["pane", "read", &lane.pane_id, "--lines", "40"],
        None,
    )?;
    let baseline_context = current_codex_context_percent(&baseline_pane);
    let outcome = prompt_agent_with_tracker_probe(
        agent_name,
        &lane.pane_id,
        &prompt,
        "recovered startup",
        baseline_context,
        || tracker_outcome(repo, bead_id),
    )?;
    if let PromptOutcome::Settled(settled) | PromptOutcome::TrackerObserved { settled, .. } =
        &outcome
    {
        println!("{}", settled.trim_end());
    }
    Ok(outcome)
}

/// Probe the worker outcome after the shared prompt seam has completed its
/// one allowed startup recovery.
pub fn lane_settle(repo: &Path, bead: &ReadyBead) -> Result<BeadOutcome, String> {
    probe_bead_outcome(repo, &bead.id)
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

pub fn lane_reap_for_state(state: LaneState, lane: &Lane) -> Result<bool, String> {
    match lane_reap_policy(state) {
        LaneReapPolicy::Never => Ok(false),
        LaneReapPolicy::CleanOnly => lane_reap_blocked(lane),
        LaneReapPolicy::ForceAllowed => {
            lane_reap(BeadOutcome::Completed, lane)?;
            Ok(true)
        }
    }
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

    fn inputs(
        bead_outcome: BeadOutcome,
        worker_active: bool,
        pull_request: Option<&PullRequestProbe>,
    ) -> LaneStateInputs<'_> {
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
    fn reap_policy_by_lane_state() {
        assert_eq!(
            lane_reap_policy(LaneState::Merged),
            LaneReapPolicy::ForceAllowed
        );
        assert_eq!(
            lane_reap_policy(LaneState::Blocked),
            LaneReapPolicy::CleanOnly
        );
        for state in [
            LaneState::Authoring,
            LaneState::AwaitingReview,
            LaneState::ReworkRequested,
            LaneState::Stalled,
        ] {
            assert_eq!(
                lane_reap_policy(state),
                LaneReapPolicy::Never,
                "{state:?} must remain warm"
            );
        }
    }

    #[test]
    fn zero_effect_settle_nudges_before_pasted_composer_renders() {
        let pane = "› Ask Codex to do anything\n\n  gpt-5.6-sol high · Context 0% used\n";
        assert!(should_nudge_after_settle(Some(0), pane));
        assert!(!pasted_composer_is_visible(pane));
    }

    #[test]
    fn zero_effect_settle_nudges_when_pasted_composer_is_visible() {
        let pane = "› [Pasted Content 1004 chars]\n\n  gpt-5.6-sol high · Context 0% used\n";
        assert!(should_nudge_after_settle(Some(0), pane));
        assert!(pasted_composer_is_visible(pane));
    }

    #[test]
    fn engaged_settle_does_not_nudge() {
        assert!(!should_nudge_after_settle(
            Some(0),
            "› Ask Codex to do anything\n\n  gpt-5.6-sol high · Context 24% used\n"
        ));
        assert!(!should_nudge_after_settle(
            Some(0),
            "• Worker completed and reported the literal diagnostic Context 0% used\n\n\
             gpt-5.6-sol high fast · /workspace · Approve for me · Context 24% used · weekly 92% left\n"
        ));
    }

    #[test]
    fn unchanged_warm_context_is_a_zero_effect_settle() {
        assert!(should_nudge_after_settle(
            Some(24),
            "› [Pasted Content 733 chars]\n\n  gpt-5.6-sol high · Context 24% used\n"
        ));
    }

    #[test]
    fn unavailable_baseline_fails_toward_recovery() {
        assert!(should_nudge_after_settle(
            None,
            "› Ask Codex to do anything\n\n  gpt-5.6-sol high · Context 0% used\n"
        ));
        assert!(should_nudge_after_settle(
            None,
            "› Ask Codex to do anything\n\n  gpt-5.6-sol high · Context 24% used\n"
        ));
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

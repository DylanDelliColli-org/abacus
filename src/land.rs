//! Pure policy for `abacus land`.
//!
//! Process execution belongs to the binary. This module accepts captured CLI
//! output or already-known policy inputs and returns typed classifications.

use std::collections::HashSet;
use std::fmt;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Eligibility {
    Eligible,
    Ineligible { reason: String },
}

#[derive(Debug, Deserialize)]
struct Ruleset {
    #[serde(default)]
    enforcement: Option<String>,
    #[serde(default)]
    rules: Vec<RulesetRule>,
}

#[derive(Debug, Deserialize)]
struct RulesetRule {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    parameters: serde_json::Value,
}

/// Classify the array returned by GitHub's repository-rulesets endpoint.
pub fn parse_eligibility(json: &str) -> Result<Eligibility, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("unparseable GitHub ruleset payload: {e}"))?;
    if !value.is_array() {
        return Err("expected GitHub ruleset payload to be an array".into());
    }
    let rulesets: Vec<Ruleset> = serde_json::from_value(value)
        .map_err(|e| format!("unparseable GitHub ruleset payload: {e}"))?;

    let active_rules = rulesets
        .iter()
        .filter(|ruleset| ruleset.enforcement.as_deref() == Some("active"))
        .flat_map(|ruleset| &ruleset.rules);
    let rules: Vec<&RulesetRule> = active_rules.collect();

    if !rules.iter().any(|rule| rule.kind == "merge_queue") {
        return Ok(Eligibility::Ineligible {
            reason: "repository has no active merge queue rule".into(),
        });
    }

    let required_checks_present = rules.iter().any(|rule| {
        rule.kind == "required_status_checks"
            && rule.parameters["required_status_checks"]
                .as_array()
                .is_some_and(|checks| !checks.is_empty())
    });
    if !required_checks_present {
        return Ok(Eligibility::Ineligible {
            reason: "merge queue has no required checks configured".into(),
        });
    }

    Ok(Eligibility::Eligible)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub bead_id: String,
    pub branch: String,
}

#[derive(Debug, Deserialize)]
struct OpenPullRequest {
    #[serde(rename = "headRefName")]
    head_ref_name: String,
}

#[derive(Debug, Deserialize)]
struct BrListEnvelope {
    issues: Vec<ListedBead>,
}

#[derive(Debug, Deserialize)]
struct ListedBead {
    id: String,
    status: String,
}

/// Parse the object envelope emitted by `br list --json`.
///
/// This is deliberately distinct from `br ready --json`, whose root is a
/// bare array.
pub fn parse_closed_bead_ids(json: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("unparseable `br list --json` envelope: {e}"))?;
    if !value.is_object() {
        return Err(
            "expected `br list --json` envelope object with an `issues` field; got a bare array"
                .into(),
        );
    }
    let envelope: BrListEnvelope = serde_json::from_value(value)
        .map_err(|e| format!("unparseable `br list --json` envelope: {e}"))?;

    let mut ids = Vec::new();
    for bead in envelope.issues {
        if bead.id.trim().is_empty() {
            return Err("unparseable `br list --json` envelope: bead id is empty".into());
        }
        if bead.status == "closed" {
            ids.push(bead.id);
        }
    }
    Ok(ids)
}

/// Intersect open `lane/*` pull requests with closed beads.
///
/// GitHub order is retained, non-lane branches are ignored, and a closed bead
/// with no open pull request is simply absent from the successful result.
pub fn enumerate_candidates(
    open_prs_json: &str,
    closed_beads_json: &str,
) -> Result<Vec<Candidate>, String> {
    let open_prs: Vec<OpenPullRequest> = serde_json::from_str(open_prs_json)
        .map_err(|e| format!("unparseable `gh pr list --json headRefName` output: {e}"))?;
    let closed_ids = parse_closed_bead_ids(closed_beads_json)?;
    let closed: HashSet<&str> = closed_ids.iter().map(String::as_str).collect();
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    for pr in open_prs {
        let Some(bead_id) = pr.head_ref_name.strip_prefix("lane/") else {
            continue;
        };
        if bead_id.is_empty() || !closed.contains(bead_id) || !seen.insert(bead_id.to_owned()) {
            continue;
        }
        candidates.push(Candidate {
            bead_id: bead_id.to_owned(),
            branch: pr.head_ref_name,
        });
    }
    Ok(candidates)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositionResult {
    Clean,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFailure {
    pub tool: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalLeg {
    NotRun,
    Pass,
    Fail(ValidationFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionInput {
    Admission {
        composition: CompositionResult,
        local_leg: LocalLeg,
        admitted_head_sha: String,
    },
    Dequeued {
        attempts: u8,
    },
    Readmission {
        composition: CompositionResult,
        local_leg: LocalLeg,
        admitted_head_sha: String,
        attempts: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LandDecision {
    Enqueue { admitted_head_sha: String },
    Resolve,
    Park,
}

pub const MAX_RESOLUTION_ATTEMPTS: u8 = 1;

pub fn resolution_dispatch_allowed(completed_attempts: u8) -> bool {
    completed_attempts < MAX_RESOLUTION_ATTEMPTS
}

/// Apply the admission and one-attempt resolution decision table.
pub fn decide(input: DecisionInput) -> LandDecision {
    match input {
        DecisionInput::Admission {
            composition: CompositionResult::Conflict,
            ..
        } => LandDecision::Resolve,
        DecisionInput::Admission {
            composition: CompositionResult::Clean,
            local_leg: LocalLeg::Pass,
            admitted_head_sha,
        } => LandDecision::Enqueue { admitted_head_sha },
        DecisionInput::Admission { .. } => LandDecision::Park,
        DecisionInput::Dequeued { attempts } if resolution_dispatch_allowed(attempts) => {
            LandDecision::Resolve
        }
        DecisionInput::Dequeued { .. } => LandDecision::Park,
        DecisionInput::Readmission {
            composition: CompositionResult::Clean,
            local_leg: LocalLeg::Pass,
            admitted_head_sha,
            attempts: MAX_RESOLUTION_ATTEMPTS,
        } => LandDecision::Enqueue { admitted_head_sha },
        DecisionInput::Readmission { .. } => LandDecision::Park,
    }
}

/// Build the composition command without any conflict-masking strategy flags.
pub fn update_argv(default_branch: &str) -> Vec<String> {
    vec![
        "git".into(),
        "merge".into(),
        format!("origin/{default_branch}"),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueOutcome {
    Admitted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnqueueError {
    Rejected {
        code: i32,
        stdout: String,
        stderr: String,
    },
    UnexpectedSuccess {
        stdout: String,
        stderr: String,
    },
}

impl fmt::Display for EnqueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rejected {
                code,
                stdout,
                stderr,
            } => write!(
                f,
                "enqueue command exited {code}: stdout={:?}, stderr={:?}",
                stdout.trim(),
                stderr.trim()
            ),
            Self::UnexpectedSuccess { stdout, stderr } => write!(
                f,
                "enqueue command succeeded with an unknown response: stdout={:?}, stderr={:?}",
                stdout.trim(),
                stderr.trim()
            ),
        }
    }
}

impl std::error::Error for EnqueueError {}

/// Parse the `(exit code, stdout, stderr)` returned by `capture_status`.
pub fn parse_enqueue_result(
    (code, stdout, stderr): (i32, String, String),
) -> Result<EnqueueOutcome, EnqueueError> {
    if code != 0 {
        return Err(EnqueueError::Rejected {
            code,
            stdout,
            stderr,
        });
    }

    let normalized = stdout.to_ascii_lowercase();
    if normalized.contains("will be added to the merge queue")
        || normalized.contains("auto-merge enabled")
    {
        Ok(EnqueueOutcome::Admitted)
    } else {
        Err(EnqueueError::UnexpectedSuccess { stdout, stderr })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueState {
    Queued,
    Merged,
    Dequeued(String),
    Absent,
}

#[derive(Debug, Deserialize)]
struct QueueEnvelope {
    data: QueueData,
}

#[derive(Debug, Deserialize)]
struct QueueData {
    repository: Option<QueueRepository>,
}

#[derive(Debug, Deserialize)]
struct QueueRepository {
    #[serde(rename = "pullRequest")]
    pull_request: Option<QueuePullRequest>,
}

#[derive(Debug, Deserialize)]
struct QueuePullRequest {
    state: String,
    merged: bool,
    #[serde(rename = "isInMergeQueue")]
    is_in_merge_queue: bool,
    #[serde(rename = "autoMergeRequest")]
    auto_merge_request: Option<serde_json::Value>,
    #[serde(rename = "mergeQueueEntry")]
    merge_queue_entry: Option<serde_json::Value>,
    #[serde(rename = "timelineItems")]
    timeline_items: RemovalTimeline,
}

#[derive(Debug, Deserialize)]
struct RemovalTimeline {
    nodes: Vec<Option<RemovalEvent>>,
}

#[derive(Debug, Deserialize)]
struct RemovalEvent {
    reason: Option<String>,
}

/// Parse one GitHub GraphQL queue observation.
///
/// The query supplies pull-request state, current queue/auto-merge fields,
/// and the latest `RemovedFromMergeQueueEvent`. A currently queued signal
/// wins over historical removal events.
pub fn parse_queue_state(json: &str) -> Result<QueueState, String> {
    let envelope: QueueEnvelope = serde_json::from_str(json)
        .map_err(|e| format!("unparseable GitHub queue-state payload: {e}"))?;
    let Some(pr) = envelope
        .data
        .repository
        .and_then(|repository| repository.pull_request)
    else {
        return Ok(QueueState::Absent);
    };

    if pr.merged || pr.state.eq_ignore_ascii_case("MERGED") {
        return Ok(QueueState::Merged);
    }
    if pr.is_in_merge_queue || pr.merge_queue_entry.is_some() || pr.auto_merge_request.is_some() {
        return Ok(QueueState::Queued);
    }

    if let Some(removal) = pr.timeline_items.nodes.iter().flatten().next() {
        let reason = removal.reason.as_deref().unwrap_or("").trim();
        if reason.is_empty() {
            return Err("GitHub queue-state payload has an empty dequeue reason".into());
        }
        return Ok(QueueState::Dequeued(reason.to_owned()));
    }

    Ok(QueueState::Absent)
}

const STDERR_EXCERPT_CHARS: usize = 500;

fn stderr_excerpt(stderr: &str) -> String {
    let trimmed = stderr.trim();
    if trimmed.is_empty() {
        return "(no stderr captured)".into();
    }

    let mut chars = trimmed.chars();
    let excerpt: String = chars.by_ref().take(STDERR_EXCERPT_CHARS).collect();
    if chars.next().is_some() {
        format!("{excerpt}…")
    } else {
        excerpt
    }
}

pub fn admission_red_park_body(
    bead_id: &str,
    admitted_head_sha: &str,
    failure: &ValidationFailure,
) -> String {
    format!(
        "Parking bead {bead_id} at admitted head {admitted_head_sha}: local admission failed in \
         {tool}.\n\nStderr excerpt:\n{stderr}",
        tool = failure.tool,
        stderr = stderr_excerpt(&failure.stderr)
    )
}

pub fn dequeue_park_body(
    bead_id: &str,
    admitted_head_sha: &str,
    reason: &str,
) -> Result<String, String> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err("cannot build dequeue park body without a dequeue reason".into());
    }
    Ok(format!(
        "Parking bead {bead_id} at admitted head {admitted_head_sha}: resolution attempt exhausted.\n\n\
         Dequeue reason: {reason}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY_RULESETS_FIXTURE: &str = "[]";
    const QUEUE_WITHOUT_CHECKS_FIXTURE: &str = r#"[
      {
        "name": "main queue",
        "enforcement": "active",
        "rules": [
          {
            "type": "merge_queue",
            "parameters": {"max_entries_to_build": 1}
          }
        ]
      }
    ]"#;
    const EVALUATE_ONLY_RULESET_FIXTURE: &str = r#"[
      {
        "name": "main queue dry run",
        "enforcement": "evaluate",
        "rules": [
          {"type":"merge_queue","parameters":{}},
          {
            "type":"required_status_checks",
            "parameters": {
              "required_status_checks": [{"context":"test"}]
            }
          }
        ]
      }
    ]"#;

    // TODO(ab-automerge-2b2.8): wire the operator-transferred positive ruleset
    // fixture from tests/fixtures/ and assert Eligible once that live payload exists.

    const OPEN_PRS_FIXTURE: &str = r#"[
      {"headRefName":"lane/ab-a"},
      {"headRefName":"lane/ab-b"},
      {"headRefName":"feature/manual"}
    ]"#;
    const CLOSED_BEADS_FIXTURE: &str = r#"{
      "issues": [
        {"id":"ab-a","status":"closed"},
        {"id":"ab-c","status":"closed"}
      ],
      "total": 2,
      "limit": 0,
      "offset": 0,
      "has_more": false
    }"#;

    const ENQUEUE_ADDED_FIXTURE: &str = "✓ Pull request DylanDelliColli-org/abacus#31 will be added to the merge queue for main when ready\n";
    const AUTO_MERGE_ENABLED_FIXTURE: &str = "✓ Auto-merge enabled\nDylanDelliColli-org/abacus#31 will merge when all requirements are met\n";

    const QUEUED_FIXTURE: &str = r#"{
      "data": {"repository": {"pullRequest": {
        "state": "OPEN",
        "merged": false,
        "isInMergeQueue": true,
        "autoMergeRequest": null,
        "mergeQueueEntry": {"state":"QUEUED"},
        "timelineItems": {"nodes": []}
      }}}
    }"#;
    const AUTO_MERGE_PENDING_FIXTURE: &str = r#"{
      "data": {"repository": {"pullRequest": {
        "state": "OPEN",
        "merged": false,
        "isInMergeQueue": false,
        "autoMergeRequest": {"enabledAt":"2026-08-15T10:00:00Z"},
        "mergeQueueEntry": null,
        "timelineItems": {"nodes": []}
      }}}
    }"#;
    const MERGED_FIXTURE: &str = r#"{
      "data": {"repository": {"pullRequest": {
        "state": "MERGED",
        "merged": true,
        "isInMergeQueue": false,
        "autoMergeRequest": null,
        "mergeQueueEntry": null,
        "timelineItems": {"nodes": []}
      }}}
    }"#;
    const DEQUEUED_FIXTURE: &str = r#"{
      "data": {"repository": {"pullRequest": {
        "state": "OPEN",
        "merged": false,
        "isInMergeQueue": false,
        "autoMergeRequest": null,
        "mergeQueueEntry": null,
        "timelineItems": {"nodes": [
          {"reason":"Required status check \"clippy\" failed"}
        ]}
      }}}
    }"#;
    const ABSENT_FIXTURE: &str = r#"{
      "data": {"repository": {"pullRequest": {
        "state": "OPEN",
        "merged": false,
        "isInMergeQueue": false,
        "autoMergeRequest": null,
        "mergeQueueEntry": null,
        "timelineItems": {"nodes": []}
      }}}
    }"#;

    fn failure() -> ValidationFailure {
        ValidationFailure {
            tool: "clippy".into(),
            stderr: "error: this comparison is always false\n  --> src/lib.rs:42:5".into(),
        }
    }

    #[test]
    fn empty_rulesets_are_ineligible_without_a_merge_queue() {
        let eligibility = parse_eligibility(EMPTY_RULESETS_FIXTURE).unwrap();

        match eligibility {
            Eligibility::Ineligible { reason } => {
                assert!(reason.contains("merge queue"), "reason was {reason:?}");
            }
            Eligibility::Eligible => panic!("an empty ruleset payload must be ineligible"),
        }
    }

    #[test]
    fn queue_without_required_checks_is_ineligible() {
        let eligibility = parse_eligibility(QUEUE_WITHOUT_CHECKS_FIXTURE).unwrap();

        match eligibility {
            Eligibility::Ineligible { reason } => {
                assert!(reason.contains("required checks"), "reason was {reason:?}");
            }
            Eligibility::Eligible => panic!("a queue without required checks is unsafe"),
        }
    }

    #[test]
    fn evaluate_only_rulesets_do_not_make_a_repository_eligible() {
        let eligibility = parse_eligibility(EVALUATE_ONLY_RULESET_FIXTURE).unwrap();

        match eligibility {
            Eligibility::Ineligible { reason } => {
                assert!(reason.contains("merge queue"), "reason was {reason:?}");
            }
            Eligibility::Eligible => panic!("evaluate mode does not enforce a merge queue"),
        }
    }

    #[test]
    fn candidate_enumeration_intersects_closed_beads_with_open_lane_prs() {
        let candidates = enumerate_candidates(OPEN_PRS_FIXTURE, CLOSED_BEADS_FIXTURE).unwrap();

        assert_eq!(
            candidates,
            [Candidate {
                bead_id: "ab-a".into(),
                branch: "lane/ab-a".into(),
            }]
        );
    }

    #[test]
    fn br_list_envelope_parses_ids_and_rejects_a_bare_array_loudly() {
        assert_eq!(
            parse_closed_bead_ids(CLOSED_BEADS_FIXTURE).unwrap(),
            ["ab-a", "ab-c"]
        );

        let err = parse_closed_bead_ids(
            r#"[{"id":"ab-a","status":"closed"},{"id":"ab-c","status":"closed"}]"#,
        )
        .unwrap_err();
        assert!(err.contains("envelope"), "error was {err:?}");
        assert!(err.contains("br list --json"), "error was {err:?}");
    }

    #[test]
    fn closed_bead_without_an_open_pr_is_a_normal_skip() {
        let candidates = enumerate_candidates("[]", CLOSED_BEADS_FIXTURE);

        assert_eq!(candidates.unwrap(), []);
    }

    #[test]
    fn composition_conflict_always_resolves_regardless_of_local_leg() {
        for local_leg in [LocalLeg::NotRun, LocalLeg::Pass, LocalLeg::Fail(failure())] {
            let row = format!("ComposeConflict + {local_leg:?}");
            let decision = decide(DecisionInput::Admission {
                composition: CompositionResult::Conflict,
                local_leg,
                admitted_head_sha: "a1b2c3d".into(),
            });

            assert_eq!(decision, LandDecision::Resolve, "row {row}");
            assert!(
                !matches!(decision, LandDecision::Enqueue { .. }),
                "row {row} must never enqueue"
            );
        }
    }

    #[test]
    fn update_argv_merges_the_parameterized_default_branch_without_strategy_options() {
        for default_branch in ["trunk", "develop"] {
            let argv = update_argv(default_branch);
            let rendered = argv.join(" ");

            assert_eq!(argv, ["git", "merge", &format!("origin/{default_branch}")]);
            assert!(!rendered.contains("-X ours"), "argv was {argv:?}");
            assert!(!rendered.contains("-X theirs"), "argv was {argv:?}");
            assert!(!rendered.contains("--strategy-option"), "argv was {argv:?}");
            assert!(!rendered.contains("origin/main"), "argv was {argv:?}");
        }
    }

    #[test]
    fn both_gh_enqueue_success_shapes_are_admitted() {
        for (shape, stdout) in [
            ("added-to-merge-queue", ENQUEUE_ADDED_FIXTURE),
            ("auto-merge-enabled", AUTO_MERGE_ENABLED_FIXTURE),
        ] {
            assert_eq!(
                parse_enqueue_result((0, stdout.into(), String::new())).unwrap(),
                EnqueueOutcome::Admitted,
                "stdout shape {shape}"
            );
        }
    }

    #[test]
    fn nonzero_enqueue_ineligibility_carries_the_exit_code() {
        let err = parse_enqueue_result((
            1,
            String::new(),
            "GraphQL: Pull request is not mergeable (mergePullRequest)".into(),
        ))
        .unwrap_err();

        assert_eq!(
            err,
            EnqueueError::Rejected {
                code: 1,
                stdout: String::new(),
                stderr: "GraphQL: Pull request is not mergeable (mergePullRequest)".into(),
            }
        );
    }

    #[test]
    fn queue_state_parser_distinguishes_queued_merged_dequeued_and_absent() {
        let cases = [
            ("queued", QUEUED_FIXTURE, QueueState::Queued),
            (
                "auto-merge pending",
                AUTO_MERGE_PENDING_FIXTURE,
                QueueState::Queued,
            ),
            ("merged", MERGED_FIXTURE, QueueState::Merged),
            (
                "dequeued",
                DEQUEUED_FIXTURE,
                QueueState::Dequeued("Required status check \"clippy\" failed".into()),
            ),
            ("absent", ABSENT_FIXTURE, QueueState::Absent),
        ];

        for (name, fixture, expected) in cases {
            assert_eq!(parse_queue_state(fixture).unwrap(), expected, "{name}");
        }
    }

    #[test]
    fn dequeued_queue_state_requires_a_nonempty_reason() {
        let fixture =
            DEQUEUED_FIXTURE.replace("Required status check \\\"clippy\\\" failed", "   ");

        let err = parse_queue_state(&fixture).unwrap_err();
        assert!(err.contains("dequeue reason"), "error was {err:?}");
    }

    #[test]
    fn only_clean_composition_with_a_passing_local_leg_enqueues_the_head_sha() {
        let decision = decide(DecisionInput::Admission {
            composition: CompositionResult::Clean,
            local_leg: LocalLeg::Pass,
            admitted_head_sha: "admitted123".into(),
        });

        assert_eq!(
            decision,
            LandDecision::Enqueue {
                admitted_head_sha: "admitted123".into(),
            }
        );
    }

    #[test]
    fn every_non_enqueue_decision_row_is_asserted_by_name() {
        let cases = [
            (
                "ComposeConflict + any local leg",
                DecisionInput::Admission {
                    composition: CompositionResult::Conflict,
                    local_leg: LocalLeg::NotRun,
                    admitted_head_sha: "conflict123".into(),
                },
                LandDecision::Resolve,
            ),
            (
                "ComposeClean + LocalFail",
                DecisionInput::Admission {
                    composition: CompositionResult::Clean,
                    local_leg: LocalLeg::Fail(failure()),
                    admitted_head_sha: "red123".into(),
                },
                LandDecision::Park,
            ),
            (
                "Dequeued + attempts=0",
                DecisionInput::Dequeued { attempts: 0 },
                LandDecision::Resolve,
            ),
            (
                "Dequeued + attempts=1",
                DecisionInput::Dequeued { attempts: 1 },
                LandDecision::Park,
            ),
        ];

        for (row, input, expected) in cases {
            let decision = decide(input);
            assert_eq!(decision, expected, "row {row}");
            assert!(
                !matches!(decision, LandDecision::Enqueue { .. }),
                "row {row} must not enqueue"
            );
        }
    }

    #[test]
    fn one_failed_agent_attempt_parks_and_never_allows_a_second_dispatch() {
        assert!(resolution_dispatch_allowed(0));
        assert!(!resolution_dispatch_allowed(1));
        assert!(!resolution_dispatch_allowed(2));
        assert_eq!(
            decide(DecisionInput::Dequeued { attempts: 1 }),
            LandDecision::Park
        );
    }

    #[test]
    fn failed_local_readmission_after_resolution_parks_never_enqueues() {
        let decision = decide(DecisionInput::Readmission {
            composition: CompositionResult::Clean,
            local_leg: LocalLeg::Fail(failure()),
            admitted_head_sha: "resolved456".into(),
            attempts: 1,
        });

        assert_eq!(decision, LandDecision::Park);
        assert!(!matches!(decision, LandDecision::Enqueue { .. }));
    }

    #[test]
    fn park_bodies_carry_the_decided_admission_and_dequeue_evidence() {
        let admission = admission_red_park_body("ab-a", "admitted123", &failure());
        assert!(admission.contains("ab-a"), "body was {admission:?}");
        assert!(admission.contains("admitted123"), "body was {admission:?}");
        assert!(admission.contains("clippy"), "body was {admission:?}");
        assert!(
            admission.contains("this comparison is always false"),
            "body was {admission:?}"
        );

        let dequeue = dequeue_park_body(
            "ab-a",
            "admitted123",
            "Required status check \"clippy\" failed",
        )
        .unwrap();
        assert!(dequeue.contains("ab-a"), "body was {dequeue:?}");
        assert!(dequeue.contains("admitted123"), "body was {dequeue:?}");
        assert!(
            dequeue.contains("Required status check \"clippy\" failed"),
            "body was {dequeue:?}"
        );
        assert!(
            dequeue.contains("attempt exhausted"),
            "body was {dequeue:?}"
        );
    }
}

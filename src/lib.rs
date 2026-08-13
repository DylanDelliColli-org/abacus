//! Core logic for `abacus run`: parse the substrate CLIs' JSON output and
//! compose the dispatch prompt. All process spawning lives in `main.rs`;
//! everything here is pure so it can be tested against captured fixtures.

use serde::Deserialize;

/// One issue as emitted by `br ready --json` (an array of these).
/// Only the fields the engine acts on are modeled; br's schema carries more.
#[derive(Debug, Clone, Deserialize)]
pub struct ReadyBead {
    pub id: String,
    pub title: String,
    #[serde(default = "default_priority")]
    pub priority: i64,
}

fn default_priority() -> i64 {
    2
}

pub fn parse_ready(json: &str) -> Result<Vec<ReadyBead>, String> {
    serde_json::from_str(json).map_err(|e| format!("unparseable `br ready --json` output: {e}"))
}

/// Lowest priority number wins (br convention: 0 is most urgent).
/// Ties keep br's own output order.
pub fn select_bead(beads: &[ReadyBead]) -> Option<&ReadyBead> {
    beads.iter().min_by_key(|b| b.priority)
}

/// The evidence `abacus run` uses after Herdr says a lane has settled.
/// Herdr's agent state is only a wake-up signal; the bead status is the
/// durable account of whether the worker actually engaged and completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeadOutcome {
    Completed,
    Incomplete,
    NeverEngaged,
}

pub fn classify_bead_status(status: &str) -> Result<BeadOutcome, String> {
    match status {
        "closed" => Ok(BeadOutcome::Completed),
        "in_progress" => Ok(BeadOutcome::Incomplete),
        "open" => Ok(BeadOutcome::NeverEngaged),
        other => Err(format!("unsupported bead status {other:?}")),
    }
}

#[derive(Deserialize)]
struct BeadState {
    status: String,
}

/// Parse the one-record array emitted by `br show <id> --json` and classify
/// the worker outcome represented by its status.
pub fn parse_bead_outcome(json: &str) -> Result<BeadOutcome, String> {
    let beads: Vec<BeadState> = serde_json::from_str(json)
        .map_err(|e| format!("unparseable `br show --json` output: {e}"))?;
    let [bead] = beads.as_slice() else {
        return Err(format!(
            "expected one bead from `br show --json`, got {}",
            beads.len()
        ));
    };
    classify_bead_status(&bead.status)
}

/// The lane a `herdr worktree create` call opened.
#[derive(Debug, PartialEq)]
pub struct Lane {
    pub workspace_id: String,
    pub pane_id: String,
    pub checkout_path: String,
    pub branch: String,
}

/// Parse the JSON envelope `herdr worktree create` prints:
/// `{"id":"cli:worktree:create","result":{"type":"worktree_created",...}}`.
pub fn parse_worktree_created(json: &str) -> Result<Lane, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("unparseable herdr output: {e}"))?;
    let result = &v["result"];
    let kind = result["type"].as_str().unwrap_or("");
    if kind != "worktree_created" {
        return Err(format!(
            "expected result.type worktree_created, got {kind:?} in: {json}"
        ));
    }
    let field = |path: &[&str]| -> Result<String, String> {
        let mut cur = result;
        for key in path {
            cur = &cur[*key];
        }
        cur.as_str().map(str::to_owned).ok_or_else(|| {
            format!(
                "missing {} in herdr worktree_created output",
                path.join(".")
            )
        })
    };
    Ok(Lane {
        workspace_id: field(&["workspace", "workspace_id"])?,
        pane_id: field(&["root_pane", "pane_id"])?,
        checkout_path: field(&["worktree", "path"])?,
        branch: field(&["worktree", "branch"])?,
    })
}

/// The dispatch prompt is the bead's identity carriage (measured finding 3,
/// SHIFT-REPORT-2026-08-13 §7): a context-lost worker must be able to find
/// its own bead and branch from the prompt alone.
pub fn dispatch_prompt(bead_id: &str, branch: &str) -> String {
    format!(
        "You are the worker lane for bead {bead_id}. This pane's working directory is a git \
         worktree on branch {branch}; do all work here. Run `br show {bead_id}` for your full \
         scope, then `br update {bead_id} --claim`. Write the failing test first, then \
         implement until it passes, then run the full test suite. Commit, push with \
         `git push -u origin {branch}`, then run `br close {bead_id}`. If you cannot proceed, \
         say BLOCKED and why, and stop."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Captured live from `br ready --json` in this repository, 2026-08-13.
    const BR_READY_FIXTURE: &str = r#"[
      {
        "created_at": "2026-08-13T13:58:29.876780198Z",
        "created_by": "ddc",
        "description": "Write CONSTRAINTS.md at the repo root.",
        "id": "abacus-vkd",
        "issue_type": "task",
        "priority": 2,
        "status": "open",
        "title": "CONSTRAINTS.md: carry the four measured findings",
        "updated_at": "2026-08-13T13:58:29.876780198Z"
      }
    ]"#;

    // Captured live from `herdr worktree create` (probe lane), 2026-08-13.
    const WORKTREE_CREATED_FIXTURE: &str = r#"{"id":"cli:worktree:create","result":{"root_pane":{"agent_status":"unknown","cwd":"/home/ddc/.herdr/worktrees/abacus/lane-probe","focused":false,"foreground_cwd":"/home/ddc/.herdr/worktrees/abacus/lane-probe","pane_id":"w1N:p1","revision":0,"scroll":{"max_offset_from_bottom":0,"offset_from_bottom":0,"viewport_rows":66},"tab_id":"w1N:t1","terminal_id":"term_658edfbaec69060","workspace_id":"w1N"},"tab":{"agent_status":"unknown","focused":false,"label":"1","number":1,"pane_count":1,"tab_id":"w1N:t1","workspace_id":"w1N"},"type":"worktree_created","workspace":{"active_tab_id":"w1N:t1","agent_status":"unknown","focused":false,"label":"probe","number":6,"pane_count":1,"tab_count":1,"workspace_id":"w1N","worktree":{"checkout_path":"/home/ddc/.herdr/worktrees/abacus/lane-probe","is_linked_worktree":true,"repo_key":"/home/ddc/dev-environment/abacus/.git","repo_name":"abacus","repo_root":"/home/ddc/dev-environment/abacus"}},"worktree":{"branch":"lane/probe","is_bare":false,"is_detached":false,"is_linked_worktree":true,"is_prunable":false,"label":"abacus","open_workspace_id":"w1N","path":"/home/ddc/.herdr/worktrees/abacus/lane-probe"}}}"#;

    #[test]
    fn parses_live_ready_fixture() {
        let beads = parse_ready(BR_READY_FIXTURE).unwrap();
        assert_eq!(beads.len(), 1);
        assert_eq!(beads[0].id, "abacus-vkd");
        assert_eq!(beads[0].priority, 2);
        assert_eq!(
            beads[0].title,
            "CONSTRAINTS.md: carry the four measured findings"
        );
    }

    #[test]
    fn empty_ready_selects_nothing() {
        let beads = parse_ready("[]").unwrap();
        assert!(select_bead(&beads).is_none());
    }

    #[test]
    fn garbage_ready_is_an_error() {
        assert!(parse_ready("Error: Beads not initialized").is_err());
    }

    #[test]
    fn selection_prefers_lowest_priority_number_then_br_order() {
        let beads = parse_ready(
            r#"[
              {"id":"abacus-aaa","title":"later","priority":2},
              {"id":"abacus-bbb","title":"urgent","priority":1},
              {"id":"abacus-ccc","title":"urgent too","priority":1}
            ]"#,
        )
        .unwrap();
        assert_eq!(select_bead(&beads).unwrap().id, "abacus-bbb");
    }

    #[test]
    fn missing_priority_defaults_to_two() {
        let beads = parse_ready(r#"[{"id":"abacus-x","title":"t"}]"#).unwrap();
        assert_eq!(beads[0].priority, 2);
    }

    #[test]
    fn parses_live_worktree_created_fixture() {
        let lane = parse_worktree_created(WORKTREE_CREATED_FIXTURE).unwrap();
        assert_eq!(
            lane,
            Lane {
                workspace_id: "w1N".into(),
                pane_id: "w1N:p1".into(),
                checkout_path: "/home/ddc/.herdr/worktrees/abacus/lane-probe".into(),
                branch: "lane/probe".into(),
            }
        );
    }

    #[test]
    fn wrong_result_type_is_an_error() {
        let err = parse_worktree_created(r#"{"id":"x","result":{"type":"worktree_removed"}}"#)
            .unwrap_err();
        assert!(err.contains("worktree_removed"));
    }

    #[test]
    fn missing_pane_is_a_named_error() {
        let err = parse_worktree_created(
            r#"{"id":"x","result":{"type":"worktree_created","workspace":{"workspace_id":"w9"}}}"#,
        )
        .unwrap_err();
        assert!(err.contains("root_pane.pane_id"), "got: {err}");
    }

    #[test]
    fn dispatch_prompt_carries_bead_identity_and_protocol() {
        let p = dispatch_prompt("abacus-v8s", "lane/abacus-v8s");
        assert!(p.contains("abacus-v8s"));
        assert!(p.contains("lane/abacus-v8s"));
        assert!(p.contains("br show abacus-v8s"));
        assert!(p.contains("br close abacus-v8s"));
        assert!(p.contains("git push -u origin lane/abacus-v8s"));
    }

    #[test]
    fn bead_status_classifies_the_three_worker_outcomes() {
        assert_eq!(
            classify_bead_status("closed").unwrap(),
            BeadOutcome::Completed
        );
        assert_eq!(
            classify_bead_status("in_progress").unwrap(),
            BeadOutcome::Incomplete
        );
        assert_eq!(
            classify_bead_status("open").unwrap(),
            BeadOutcome::NeverEngaged
        );
    }
}

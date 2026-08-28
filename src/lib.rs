//! Core logic for `abacus`: parse substrate CLI output, compose dispatch
//! prompts, and run the named lane-lifecycle phases.

pub mod land;
pub mod lane;
pub mod review;

use serde::Deserialize;
use std::path::Path;

pub const OPERATOR_SEAT_LABEL: &str = "seat:operator";

/// The crate version embedded by Cargo at compile time.
pub fn version_string() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Format a lane's elapsed wall-clock seconds for compact outcome messages.
pub fn format_lane_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m{:02}s", secs / 60, secs % 60)
    }
}

/// One issue as emitted by `br ready --json` (an array of these).
/// Only the fields the engine acts on are modeled; br's schema carries more.
#[derive(Debug, Clone, Deserialize)]
pub struct ReadyBead {
    pub id: String,
    pub title: String,
    pub issue_type: String,
    #[serde(default = "default_priority")]
    pub priority: i64,
    #[serde(default)]
    pub labels: Vec<String>,
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
    beads
        .iter()
        .filter(|bead| {
            bead.issue_type != "epic"
                && !bead.labels.iter().any(|label| label == OPERATOR_SEAT_LABEL)
        })
        .min_by_key(|bead| bead.priority)
}

const HERDR_AGENT_NAME_LIMIT: usize = 32;
const AGENT_NAME_HASH_HEX_LEN: usize = 8;

fn agent_name_hash(value: &str) -> u32 {
    value.as_bytes().iter().fold(0x811c9dc5, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(0x01000193)
    })
}

fn normalize_agent_name(value: &str) -> String {
    let mut name: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_') {
                c
            } else {
                '-'
            }
        })
        .collect();

    match name.as_bytes().first() {
        Some(b'a'..=b'z') => {}
        Some(_) => name.replace_range(..1, "a"),
        None => name.push('a'),
    }
    name
}

/// Sanitize an agent-name stem while reserving a short, grammar-safe suffix.
/// If the combined identity needs truncation, its full hash remains intact
/// immediately before the suffix.
pub(crate) fn sanitize_agent_name_with_reserved_suffix(value: &str, suffix: &str) -> String {
    let mut name = normalize_agent_name(value);
    let identity = format!("{value}{suffix}");
    let reserved = suffix.len() + AGENT_NAME_HASH_HEX_LEN + 1;

    if reserved >= HERDR_AGENT_NAME_LIMIT {
        return sanitize_agent_name(&identity);
    }
    if name.len() + suffix.len() > HERDR_AGENT_NAME_LIMIT {
        let hash = agent_name_hash(&identity);
        name.truncate(HERDR_AGENT_NAME_LIMIT - reserved);
        name.push('-');
        name.push_str(&format!("{hash:08x}"));
    }
    name.push_str(suffix);
    name
}

/// Convert a bead id into Herdr's display-name grammar:
/// `[a-z][a-z0-9_-]{0,31}`. Unsupported characters become hyphens; the
/// leading position is normalized separately so every input remains valid.
/// Names that need truncation retain a stable hash of the full bead id.
pub fn sanitize_agent_name(bead_id: &str) -> String {
    let mut name = normalize_agent_name(bead_id);
    if name.len() > HERDR_AGENT_NAME_LIMIT {
        let hash = agent_name_hash(bead_id);
        name.truncate(HERDR_AGENT_NAME_LIMIT - AGENT_NAME_HASH_HEX_LEN - 1);
        name.push('-');
        name.push_str(&format!("{hash:08x}"));
    }
    name
}

/// The evidence `abacus run` uses after Herdr says a lane has settled.
/// Herdr's agent state is only a wake-up signal; the bead status is the
/// durable account of whether the worker actually engaged and completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeadOutcome {
    Completed,
    Incomplete,
    Blocked,
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

/// A completed bead is durable evidence that its lane can be torn down.
/// Incomplete and never-engaged lanes stay open so their pane transcripts
/// remain available for diagnosis.
pub fn should_reap_lane(outcome: BeadOutcome) -> bool {
    outcome == BeadOutcome::Completed
}

/// Whether a failed `herdr agent prompt --wait` is the observed startup race:
/// the prompt reached the terminal before the agent TUI attached and Herdr
/// saw an idle agent with no state transition.
pub fn is_agent_prompt_stalled(error: &str) -> bool {
    error.contains("agent_prompt_stalled")
        || (error.contains("agent prompt produced no observed state change")
            && error.contains("status is idle"))
}

/// Whether `herdr worktree remove` refused a dirty checkout that may be
/// retried explicitly with `--force`.
pub fn is_dirty_worktree_remove_error(error: &str) -> bool {
    error.contains("dirty_worktree_requires_force")
}

#[derive(Deserialize)]
struct BeadState {
    status: String,
    #[serde(default)]
    comments: Vec<BeadComment>,
}

#[derive(Deserialize)]
struct BeadComment {
    id: i64,
    text: String,
}

fn has_blocked_leading_token(text: &str) -> bool {
    let Some(remainder) = text.strip_prefix(review::BLOCKED_COMMENT_TOKEN) else {
        return false;
    };
    remainder
        .chars()
        .next()
        .is_none_or(|boundary| !boundary.is_alphanumeric() && boundary != '_')
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
    if bead.status == "in_progress"
        && bead
            .comments
            .iter()
            .max_by_key(|comment| comment.id)
            .is_some_and(|comment| has_blocked_leading_token(&comment.text))
    {
        Ok(BeadOutcome::Blocked)
    } else {
        classify_bead_status(&bead.status)
    }
}

/// The lane a Herdr worktree command opened.
#[derive(Debug, PartialEq)]
pub struct Lane {
    pub workspace_id: String,
    pub pane_id: String,
    pub checkout_path: String,
    pub branch: String,
}

fn parse_worktree_result(json: &str, expected_kind: &str) -> Result<Lane, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("unparseable herdr output: {e}"))?;
    let result = &v["result"];
    let kind = result["type"].as_str().unwrap_or("");
    if kind != expected_kind {
        return Err(format!(
            "expected result.type {expected_kind}, got {kind:?} in: {json}"
        ));
    }
    let field = |path: &[&str]| -> Result<String, String> {
        let mut cur = result;
        for key in path {
            cur = &cur[*key];
        }
        cur.as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("missing {} in herdr {expected_kind} output", path.join(".")))
    };
    Ok(Lane {
        workspace_id: field(&["workspace", "workspace_id"])?,
        pane_id: field(&["root_pane", "pane_id"])?,
        checkout_path: field(&["worktree", "path"])?,
        branch: field(&["worktree", "branch"])?,
    })
}

/// Parse the JSON envelope `herdr worktree create` prints:
/// `{"id":"cli:worktree:create","result":{"type":"worktree_created",...}}`.
pub fn parse_worktree_created(json: &str) -> Result<Lane, String> {
    parse_worktree_result(json, "worktree_created")
}

/// Parse the JSON envelope `herdr worktree open` prints.
pub fn parse_worktree_opened(json: &str) -> Result<Lane, String> {
    parse_worktree_result(json, "worktree_opened")
}

/// Read the Rust MSRV declared by a target repository, if it has one.
pub fn target_rust_version(repo: &Path) -> Result<Option<String>, String> {
    let manifest_path = repo.join("Cargo.toml");
    let manifest = match std::fs::read_to_string(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "failed to read target manifest {}: {error}",
                manifest_path.display()
            ));
        }
    };

    Ok(manifest.lines().find_map(|line| {
        let (key, value) = line.trim().split_once('=')?;
        if key.trim() != "rust-version" {
            return None;
        }
        let value = value.trim();
        value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .map(str::to_owned)
    }))
}

/// The dispatch prompt is the bead's identity carriage (measured finding 3,
/// SHIFT-REPORT-2026-08-13 §7): a context-lost worker must be able to find
/// its own bead and branch from the prompt alone. Target-specific manifest
/// discovery happens outside this pure builder.
pub fn dispatch_prompt(
    bead_id: &str,
    branch: &str,
    default_branch: &str,
    rust_version: Option<&str>,
) -> String {
    let verification = rust_version.map_or_else(
        || "Then run the full test suite.".to_owned(),
        |version| {
            format!(
                "Pin verification to the target workspace MSRV: if needed, install it once with \
                 `rustup toolchain install {version} --profile minimal --component clippy --component rustfmt`; \
                 then run the full test suite with `RUSTUP_TOOLCHAIN={version} cargo test`, \
                 `RUSTUP_TOOLCHAIN={version} cargo clippy --all-targets --all-features -- -D warnings`, \
                 and `RUSTUP_TOOLCHAIN={version} cargo fmt --check`."
            )
        },
    );

    format!(
        "You are the worker lane for bead {bead_id}. This pane's working directory is a git \
         worktree on branch {branch}; do all work here. The bead is already claimed to this lane. \
         Run `br show {bead_id}` for your full scope. Write the failing test first, then implement \
         until it passes. {verification} Once verification passes, commit all work (source and test \
         changes only), and push with `git push -u origin {branch}`. After the push, run \
         `gh pr create --base {default_branch}`; use a title containing `{bead_id}` and write your own body \
         summarizing what was done and the test evidence, including suite results and red-first \
         confirmation. If a PR already exists for `{branch}`, treat that existing PR as success \
         rather than a blocker. Only after the push has succeeded and the PR exists, run \
         `br close {bead_id}` as your final act. Verify the worktree is clean. If you cannot \
         proceed, say {blocked_token} and why, and stop.",
        blocked_token = review::BLOCKED_COMMENT_TOKEN,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_PROMPT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    struct PromptFixture(PathBuf);

    impl PromptFixture {
        fn new(rust_version: Option<&str>) -> Self {
            let sequence = NEXT_PROMPT_FIXTURE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "abacus-prompt-fixture-{}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).unwrap();
            let fixture = Self(path);
            if let Some(version) = rust_version {
                fixture.write_manifest(version);
            }
            fixture
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn write_manifest(&self, rust_version: &str) {
            std::fs::write(
                self.path().join("Cargo.toml"),
                format!("[package]\nname = \"target-fixture\"\nversion = \"0.0.0\"\nrust-version = \"{rust_version}\"\n"),
            )
            .unwrap();
        }
    }

    impl Drop for PromptFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn version_string_matches_cargo_package_version() {
        assert_eq!(version_string(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn lane_duration_formats_seconds_and_zero_padded_minutes() {
        assert_eq!(format_lane_duration(0), "0s");
        assert_eq!(format_lane_duration(42), "42s");
        assert_eq!(format_lane_duration(4 * 60 + 7), "4m07s");
    }

    // Representative `br 0.3.x ready --json` output with inline labels.
    const BR_READY_FIXTURE: &str = r#"[
      {
        "created_at": "2026-08-13T13:58:29.876780198Z",
        "created_by": "ddc",
        "description": "Write CONSTRAINTS.md at the repo root.",
        "id": "abacus-vkd",
        "issue_type": "task",
        "labels": ["documentation"],
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
        assert_eq!(beads[0].issue_type, "task");
        assert_eq!(beads[0].labels, ["documentation"]);
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
              {"id":"abacus-aaa","title":"later","issue_type":"task","priority":2},
              {"id":"abacus-bbb","title":"urgent","issue_type":"task","priority":1},
              {"id":"abacus-ccc","title":"urgent too","issue_type":"task","priority":1}
            ]"#,
        )
        .unwrap();
        assert_eq!(select_bead(&beads).unwrap().id, "abacus-bbb");
    }

    #[test]
    fn selection_skips_operator_seat_beads_from_ready_labels() {
        let beads = parse_ready(
            r#"[
              {"id":"abacus-operator","title":"operator milestone","issue_type":"task","priority":0,"labels":["seat:operator"]},
              {"id":"abacus-worker","title":"worker task","issue_type":"task","priority":1}
            ]"#,
        )
        .unwrap();

        assert_eq!(select_bead(&beads).unwrap().id, "abacus-worker");
        assert!(select_bead(&beads[..1]).is_none());
    }

    #[test]
    fn selection_skips_ready_epics() {
        let beads = parse_ready(
            r#"[
              {"id":"abacus-parent","title":"planning parent","priority":0,"issue_type":"epic","labels":[]},
              {"id":"abacus-worker","title":"worker task","priority":1,"issue_type":"task","labels":[]}
            ]"#,
        )
        .unwrap();

        assert_eq!(select_bead(&beads).unwrap().id, "abacus-worker");
        assert!(select_bead(&beads[..1]).is_none());
    }

    #[test]
    fn missing_priority_defaults_to_two() {
        let beads = parse_ready(r#"[{"id":"abacus-x","title":"t","issue_type":"task"}]"#).unwrap();
        assert_eq!(beads[0].priority, 2);
    }

    #[test]
    fn sanitizes_bead_ids_for_herdr_agent_names() {
        assert_eq!(sanitize_agent_name("ab-qmc.1"), "ab-qmc-1");
        assert_eq!(sanitize_agent_name("ab-QMC.1"), "ab-----1");
        let truncated = sanitize_agent_name("abcdefghijklmnopqrstuvwxyz0123456789");
        let (prefix, hash) = truncated.rsplit_once('-').unwrap();
        assert_eq!(prefix, "abcdefghijklmnopqrstuvw");
        assert_eq!(hash.len(), AGENT_NAME_HASH_HEX_LEN);
        assert!(hash.chars().all(|character| character.is_ascii_hexdigit()));

        for bead_id in [
            "ab-qmc.1",
            "ab-QMC.1",
            "abcdefghijklmnopqrstuvwxyz0123456789",
        ] {
            let name = sanitize_agent_name(bead_id);
            assert!((1..=32).contains(&name.len()), "name was {name:?}");
            assert!(
                name.starts_with(|c: char| c.is_ascii_lowercase()),
                "name was {name:?}"
            );
            assert!(
                name.chars().all(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_')
                }),
                "name was {name:?}"
            );
        }
    }

    #[test]
    fn truncated_agent_names_use_the_full_bead_id_to_avoid_collisions() {
        let first = sanitize_agent_name("market-brief-package-aywst.14.4.15");
        let second = sanitize_agent_name("market-brief-package-aywst.14.4.18");

        assert_ne!(first, second);
        assert_eq!(first.len(), 32);
        assert_eq!(second.len(), 32);
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
    fn parses_live_worktree_opened_fixture() {
        let lane = parse_worktree_opened(
            r#"{"result":{"type":"worktree_opened","workspace":{"workspace_id":"w2N"},"root_pane":{"pane_id":"w2N:p1"},"worktree":{"path":"/repo/lane-open","branch":"lane/ab-open"}}}"#,
        )
        .unwrap();

        assert_eq!(
            lane,
            Lane {
                workspace_id: "w2N".into(),
                pane_id: "w2N:p1".into(),
                checkout_path: "/repo/lane-open".into(),
                branch: "lane/ab-open".into(),
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
        let p = dispatch_prompt("abacus-v8s", "lane/abacus-v8s", "main", None);
        assert!(p.contains("abacus-v8s"));
        assert!(p.contains("lane/abacus-v8s"));
        assert!(p.contains("br show abacus-v8s"));
        assert!(p.contains("already claimed to this lane"));
        assert!(p.contains("br close abacus-v8s"));
        assert!(p.contains("git push -u origin lane/abacus-v8s"));
        assert!(p.contains("gh pr create --base main"));
        assert!(p.contains("title containing `abacus-v8s`"));
        assert!(p.contains("suite results"));
        assert!(p.contains("red-first confirmation"));
        assert!(p.contains("already exists for `lane/abacus-v8s`"));
        assert!(p.contains("treat that existing PR as success"));
        assert!(
            !p.contains("--claim"),
            "the engine already claimed the shared-store bead: {p}"
        );
        assert!(
            !p.contains("git add .beads"),
            "lane commits must not carry tracker state: {p}"
        );

        let close = p.find("br close abacus-v8s").unwrap();
        let push = p.find("git push -u origin lane/abacus-v8s").unwrap();
        let pr = p.find("gh pr create --base main").unwrap();
        assert!(push < pr, "push must happen before PR creation: {p}");
        assert!(
            pr < close,
            "close must be the final act after the PR exists: {p}"
        );

        let develop = dispatch_prompt("abacus-v8s", "lane/abacus-v8s", "develop", None);
        assert!(develop.contains("gh pr create --base develop"));
    }

    #[test]
    fn prompt_pins_verification_to_the_target_manifest_msrv() {
        let target = PromptFixture::new(Some("1.82"));
        let target_msrv = target_rust_version(target.path()).unwrap().unwrap();
        let p = dispatch_prompt("abacus-v8s", "lane/abacus-v8s", "main", Some(&target_msrv));

        for expected in [
            format!("rustup toolchain install {target_msrv}"),
            format!("RUSTUP_TOOLCHAIN={target_msrv} cargo test"),
            format!(
                "RUSTUP_TOOLCHAIN={target_msrv} cargo clippy --all-targets --all-features -- -D warnings"
            ),
            format!("RUSTUP_TOOLCHAIN={target_msrv} cargo fmt --check"),
        ] {
            assert!(p.contains(&expected), "missing {expected:?} in: {p}");
        }
    }

    #[test]
    fn prompt_omits_rust_commands_without_a_manifest() {
        let target = PromptFixture::new(None);
        assert!(!target.path().join("Cargo.toml").exists());
        let target_msrv = target_rust_version(target.path()).unwrap();
        let p = dispatch_prompt(
            "abacus-v8s",
            "lane/abacus-v8s",
            "main",
            target_msrv.as_deref(),
        );

        assert!(
            !p.contains("rustup"),
            "non-Rust prompt contained rustup: {p}"
        );
        assert!(!p.contains("cargo"), "non-Rust prompt contained cargo: {p}");
        assert!(p.contains("Write the failing test first"));
        assert!(p.contains("run the full test suite"));
        assert!(p.contains("git push -u origin lane/abacus-v8s"));
        assert!(p.contains("gh pr create --base main"));
        assert!(p.contains("br close abacus-v8s"));
    }

    #[test]
    fn changing_target_manifest_msrv_changes_the_generated_prompt() {
        let target = PromptFixture::new(Some("1.82"));
        let first_msrv = target_rust_version(target.path()).unwrap().unwrap();
        let first = dispatch_prompt("abacus-v8s", "lane/abacus-v8s", "main", Some(&first_msrv));

        target.write_manifest("1.83");
        let second_msrv = target_rust_version(target.path()).unwrap().unwrap();
        let second = dispatch_prompt("abacus-v8s", "lane/abacus-v8s", "main", Some(&second_msrv));
        assert_ne!(first_msrv, second_msrv);
        assert_ne!(
            first, second,
            "target manifest change must change the prompt"
        );
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
        assert_eq!(
            parse_bead_outcome(r#"[{"status":"closed","comments":[]}]"#).unwrap(),
            BeadOutcome::Completed
        );
        assert_eq!(
            parse_bead_outcome(r#"[{"status":"in_progress","comments":[]}]"#).unwrap(),
            BeadOutcome::Incomplete
        );
        assert_eq!(
            parse_bead_outcome(r#"[{"status":"open","comments":[]}]"#).unwrap(),
            BeadOutcome::NeverEngaged
        );
        assert_eq!(
            parse_bead_outcome(
                r#"[{"status":"in_progress","comments":[{"id":1,"text":"BLOCKED: waiting"}]}]"#
            )
            .unwrap(),
            BeadOutcome::Blocked
        );
    }

    #[test]
    fn blocked_is_in_progress_with_a_blocked_leading_highest_id_comment() {
        let fixture = r#"[{"status":"in_progress","comments":[
            {"id":1,"issue_id":"ab-example","author":"worker","text":"starting","created_at":"2026-08-19T12:00:00Z"},
            {"id":2,"issue_id":"ab-example","author":"worker","text":"BLOCKED: cannot reach origin","created_at":"2026-08-19T12:00:00Z"}
        ]}]"#;

        assert_eq!(parse_bead_outcome(fixture).unwrap(), BeadOutcome::Blocked);
    }

    #[test]
    fn a_newer_non_blocked_comment_supersedes_an_older_blocked_one() {
        let ordered = r#"[{"status":"in_progress","comments":[
            {"id":1,"issue_id":"ab-example","author":"worker","text":"BLOCKED: waiting","created_at":"2026-08-19T12:00:00Z"},
            {"id":2,"issue_id":"ab-example","author":"worker","text":"unblocked, resuming","created_at":"2026-08-19T12:00:00Z"}
        ]}]"#;
        let reversed = r#"[{"status":"in_progress","comments":[
            {"id":2,"issue_id":"ab-example","author":"worker","text":"unblocked, resuming","created_at":"2026-08-19T12:00:00Z"},
            {"id":1,"issue_id":"ab-example","author":"worker","text":"BLOCKED: waiting","created_at":"2026-08-19T12:00:00Z"}
        ]}]"#;

        assert_eq!(
            parse_bead_outcome(ordered).unwrap(),
            BeadOutcome::Incomplete
        );
        assert_eq!(
            parse_bead_outcome(reversed).unwrap(),
            BeadOutcome::Incomplete
        );
    }

    #[test]
    fn comments_field_absent_or_empty_is_plain_incomplete() {
        assert_eq!(
            parse_bead_outcome(r#"[{"status":"in_progress"}]"#).unwrap(),
            BeadOutcome::Incomplete
        );
        assert_eq!(
            parse_bead_outcome(r#"[{"status":"in_progress","comments":[]}]"#).unwrap(),
            BeadOutcome::Incomplete
        );
    }

    #[test]
    fn blocked_token_is_case_sensitive_and_boundary_checked() {
        for (text, expected) in [
            ("BLOCKED: cannot reach origin", BeadOutcome::Blocked),
            ("BLOCKED — cannot reach origin", BeadOutcome::Blocked),
            ("Blocked: cannot reach origin", BeadOutcome::Incomplete),
            ("UNBLOCKED: origin reachable", BeadOutcome::Incomplete),
        ] {
            let fixture = format!(
                r#"[{{"status":"in_progress","comments":[{{"id":1,"issue_id":"ab-example","author":"worker","text":{text:?},"created_at":"2026-08-19T12:00:00Z"}}]}}]"#
            );

            assert_eq!(parse_bead_outcome(&fixture).unwrap(), expected, "{text:?}");
        }
    }

    #[test]
    fn a_closed_bead_with_a_blocked_comment_is_completed() {
        let fixture = r#"[{"status":"closed","comments":[
            {"id":1,"issue_id":"ab-example","author":"worker","text":"BLOCKED: cannot reach origin","created_at":"2026-08-19T12:00:00Z"}
        ]}]"#;

        assert_eq!(parse_bead_outcome(fixture).unwrap(), BeadOutcome::Completed);
    }

    #[test]
    fn only_a_completed_outcome_reaps_the_lane() {
        assert!(should_reap_lane(BeadOutcome::Completed));
        assert!(!should_reap_lane(BeadOutcome::Incomplete));
        assert!(!should_reap_lane(BeadOutcome::Blocked));
        assert!(!should_reap_lane(BeadOutcome::NeverEngaged));
    }

    #[test]
    fn detects_the_captured_agent_prompt_stall() {
        let captured = "agent prompt produced no observed state change within 5000 ms; \
                        status is idle and state_change_seq remained 1578.";

        assert!(is_agent_prompt_stalled(captured));
        assert!(!is_agent_prompt_stalled(
            "agent prompt failed because the agent does not exist"
        ));
    }

    #[test]
    fn detects_the_captured_dirty_worktree_removal_error() {
        let captured = r#"{"id":"cli:worktree:remove","error":{"code":"dirty_worktree_requires_force","message":"worktree contains modified or untracked files; use --force to delete it"}}"#;

        assert!(is_dirty_worktree_remove_error(captured));
        assert!(!is_dirty_worktree_remove_error(
            r#"{"id":"cli:worktree:remove","error":{"code":"worktree_remove_failed","message":"git worktree remove failed"}}"#
        ));
    }
}

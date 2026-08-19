//! Textual contracts and launch mechanics for adversarial PR review.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::lane::capture;
use crate::{is_agent_prompt_stalled, sanitize_agent_name};

pub const BLOCKED_COMMENT_TOKEN: &str = "BLOCKED";
pub const STATUS_CONTEXT: &str = "adversarial-review";

pub const VERDICT_HEADING_PREFIX: &str = "## Adversarial review — cycle ";
pub const REREVIEW_HEADING_PREFIX: &str = "# PR #";
pub const REREVIEW_HEADING_CYCLE: &str = " cycle ";
pub const REREVIEW_HEADING_SUFFIX: &str = " re-review";
pub const VERDICT_REFUTED: &str = "**Verdict REFUTED.**";
pub const VERDICT_NOT_REFUTED: &str = "**Verdict NOT REFUTED.**";
pub const PROBES_HEADING: &str = "## Probes";

pub const ADJUDICATION_HEADING_PREFIX: &str = "## Adjudication — cycle ";
pub const ADJUDICATION_ACCEPTED_VERDICT: &str = "**Verdict NOT REFUTED — accepted.**";
pub const ADJUDICATION_REWORK_VERDICT: &str = "**Verdict REFUTED — rework required.**";
pub const FINDING_ACCEPTED_PREFIX: &str = "Accepted — ";
pub const FINDING_REJECTED_PREFIX: &str = "Rejected — ";
pub const FINDING_REROUTED_PREFIX: &str = "Rerouted — ";
pub const ADJUDICATED_HEAD_PREFIX: &str = "Adjudicated head: ";

/// The stable role card appended to every dynamically scoped review brief.
pub const REFUTATION_BRIEF_TEMPLATE: &str = r#"## Read-only ground rules

Treat the target repository, branch, pull request, tracker, and agent topology as read-only. You may run read-only inspections and executed probes. The exactly one permitted write is posting your final verdict to the target PR with `gh pr comment <PR> --body-file <VERDICT_FILE>`. Do not modify source files, commits, branches, tracker state, workspaces, or agents.

Work as a fresh, maximally adversarial reviewer. Attempt to refute the bead's acceptance claims and the actual PR implementation. Convergence is a property of the author-reviewer-adjudicator system, not a reason to soften this review.

## Evidence and finding bar

- A blocker requires an executed failure or a byte-level demonstration. Speculation never blocks; a finding without either self-grades to a concern.
- Every finding must include a **Threat model** stating who can trigger it and from where. A path reachable only by a trusted producer self-grades to a concern.
- After cycle two, a new finding may block only if it belongs to a previously unadjudicated class. Otherwise identify it as follow-up work rather than a merge blocker.
- For corpus- or file-reading code, include a cwd-variance probe.

## Required verdict grammar

Begin the PR comment with the supplied adversarial-review heading. Then emit exactly one overall verdict line:

- `**Verdict REFUTED.**`
- `**Verdict NOT REFUTED.**`

For a refuted verdict, provide numbered findings. Each finding must give severity (`blocker`, `concern`, or `note`), concrete file/line evidence, refutation reasoning, its threat model, and any executed failure or byte-level demonstration. End every verdict with `## Probes` and list the commands or inspections actually performed and their outcomes.
"#;

pub struct RefutationBriefInput<'a> {
    pub bead_id: &'a str,
    pub description: &'a str,
    pub comments: &'a [String],
    pub pr_number: u64,
    pub agents_path: &'a Path,
    pub cycle: u32,
}

#[derive(Debug, Deserialize)]
struct RawReviewBead {
    #[serde(default)]
    description: String,
    #[serde(default)]
    comments: Vec<RawReviewComment>,
}

#[derive(Debug, Deserialize)]
struct RawReviewComment {
    text: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReviewBead {
    pub description: String,
    pub comments: Vec<String>,
}

pub fn parse_review_bead(json: &str) -> Result<ReviewBead, String> {
    let beads: Vec<RawReviewBead> = serde_json::from_str(json)
        .map_err(|error| format!("unparseable `br show --json` review input: {error}"))?;
    let [bead] = beads.as_slice() else {
        return Err(format!(
            "expected one bead from `br show --json` review input, got {}",
            beads.len()
        ));
    };
    Ok(ReviewBead {
        description: bead.description.clone(),
        comments: bead
            .comments
            .iter()
            .map(|comment| comment.text.clone())
            .collect(),
    })
}

pub fn verdict_heading(cycle: u32) -> String {
    format!("{VERDICT_HEADING_PREFIX}{cycle}")
}

pub fn rereview_heading(pr_number: u64, cycle: u32) -> String {
    format!(
        "{REREVIEW_HEADING_PREFIX}{pr_number}{REREVIEW_HEADING_CYCLE}{cycle}{REREVIEW_HEADING_SUFFIX}"
    )
}

pub fn reviewer_name(bead_id: &str, cycle: u32) -> String {
    let suffix = format!("-c{cycle}");
    let capacity = 32usize.saturating_sub("rev-".len() + suffix.len());
    let sanitized = sanitize_agent_name(bead_id);
    let bead_part: String = sanitized.chars().take(capacity).collect();
    format!("rev-{bead_part}{suffix}")
}

pub fn brief_path(repo: &Path, bead_id: &str, cycle: u32) -> PathBuf {
    repo.join("target/abacus-tmp/reviews")
        .join(format!("{}.md", reviewer_name(bead_id, cycle)))
}

pub fn refutation_brief(input: &RefutationBriefInput<'_>) -> String {
    let mut authority_trail = String::new();
    if input.comments.is_empty() {
        authority_trail.push_str("- No bead comments were present.\n");
    } else {
        for (index, comment) in input.comments.iter().enumerate() {
            let _ = writeln!(authority_trail, "{}. {}", index + 1, comment.trim());
        }
    }
    let heading = if input.cycle == 1 {
        verdict_heading(input.cycle)
    } else {
        rereview_heading(input.pr_number, input.cycle)
    };
    let canonical_heading = verdict_heading(input.cycle);
    let rereview_heading = rereview_heading(input.pr_number, input.cycle);
    let template = REFUTATION_BRIEF_TEMPLATE.replace(
        "gh pr comment <PR>",
        &format!("gh pr comment {}", input.pr_number),
    );

    format!(
        "# Refutation brief — bead {bead_id} — PR #{pr_number}\n\n\
         ## Authority map\n\n\
         1. Repository instructions: `{agents_path}`.\n\
         2. Bead `{bead_id}` description and acceptance contract.\n\
         3. Bead comments below, in tracker order, as the authority trail.\n\n\
         ## Per-bead refutation targets\n\n\
         {description}\n\n\
         ### Authority trail\n\n\
         {authority_trail}\n\
         Target PR: #{pr_number}. Canonical heading grammar: `{canonical_heading}`. Production \
         re-review variant: `{rereview_heading}`. Required comment heading for this cycle: \
         `{heading}`.\n\n\
         {template}",
        bead_id = input.bead_id,
        pr_number = input.pr_number,
        agents_path = input.agents_path.display(),
        description = input.description.trim(),
        canonical_heading = canonical_heading,
        rereview_heading = rereview_heading,
        template = template,
    )
}

fn parse_workspace_pane(json: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|error| format!("unparseable herdr workspace output: {error}"))?;
    value["result"]["root_pane"]["pane_id"]
        .as_str()
        .filter(|pane| !pane.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            format!("missing result.root_pane.pane_id in herdr workspace output: {json}")
        })
}

pub fn launch_reviewer(
    repo: &Path,
    bead_id: &str,
    review_bead: &ReviewBead,
    pr_number: u64,
    cycle: u32,
) -> Result<PathBuf, String> {
    let agents_path = repo.join("AGENTS.md");
    let path = brief_path(repo, bead_id, cycle);
    let parent = path
        .parent()
        .ok_or_else(|| format!("review brief has no parent directory: {}", path.display()))?;
    std::fs::create_dir_all(parent).map_err(|error| {
        format!(
            "cannot create review brief directory {}: {error}",
            parent.display()
        )
    })?;
    let brief = refutation_brief(&RefutationBriefInput {
        bead_id,
        description: &review_bead.description,
        comments: &review_bead.comments,
        pr_number,
        agents_path: &agents_path,
        cycle,
    });
    std::fs::write(&path, brief)
        .map_err(|error| format!("cannot write review brief {}: {error}", path.display()))?;

    let repo_arg = repo.to_string_lossy().into_owned();
    let agent_name = reviewer_name(bead_id, cycle);
    let opened = capture(
        "herdr",
        &[
            "workspace",
            "create",
            "--cwd",
            &repo_arg,
            "--label",
            &agent_name,
            "--no-focus",
        ],
        None,
    )?;
    let pane_id = parse_workspace_pane(&opened)?;
    capture(
        "herdr",
        &[
            "agent",
            "start",
            &agent_name,
            "--kind",
            "codex",
            "--pane",
            &pane_id,
        ],
        None,
    )?;

    let path_arg = path.to_string_lossy().into_owned();
    let prompt_args = ["agent", "prompt", &agent_name, &path_arg, "--wait"];
    match capture("herdr", &prompt_args, None) {
        Ok(_) => {}
        Err(error) if is_agent_prompt_stalled(&error) => {
            eprintln!("agent prompt stalled during reviewer startup; retrying once");
            capture("herdr", &prompt_args, None)?;
        }
        Err(error) => return Err(error),
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn refutation_brief_carries_targets_ground_rules_and_verdict_grammar() {
        let comments = vec![
            "The implementation must preserve the accepted ADR.".to_owned(),
            "Probe the branch from a different working directory.".to_owned(),
        ];
        let brief = refutation_brief(&RefutationBriefInput {
            bead_id: "ab-review.4",
            description: "Build the engine-owned adversarial review gate.",
            comments: &comments,
            pr_number: 42,
            agents_path: Path::new("/repo/AGENTS.md"),
            cycle: 3,
        });

        for required in [
            "ab-review.4",
            "#42",
            "/repo/AGENTS.md",
            "Authority map",
            "Per-bead refutation targets",
            "accepted ADR",
            "different working directory",
            "exactly one permitted write",
            "gh pr comment 42 --body-file",
            "## Adversarial review — cycle 3",
            "**Verdict REFUTED.**",
            "**Verdict NOT REFUTED.**",
            "numbered findings",
            "## Probes",
            "executed failure or a byte-level demonstration",
            "Threat model",
            "previously unadjudicated class",
            "cwd-variance probe",
        ] {
            assert!(
                brief.contains(required),
                "brief lacked {required:?}:\n{brief}"
            );
        }
        for forbidden in ["git push", "br close", "br update"] {
            assert!(
                !brief.contains(forbidden),
                "brief granted a forbidden instruction {forbidden:?}:\n{brief}"
            );
        }

        assert_eq!(BLOCKED_COMMENT_TOKEN, "BLOCKED");
        assert_eq!(STATUS_CONTEXT, "adversarial-review");
        assert_eq!(ADJUDICATION_HEADING_PREFIX, "## Adjudication — cycle ");
        assert_eq!(ADJUDICATED_HEAD_PREFIX, "Adjudicated head: ");
        assert_eq!(
            ADJUDICATION_ACCEPTED_VERDICT,
            "**Verdict NOT REFUTED — accepted.**"
        );
        assert_eq!(
            ADJUDICATION_REWORK_VERDICT,
            "**Verdict REFUTED — rework required.**"
        );
        assert_eq!(FINDING_ACCEPTED_PREFIX, "Accepted — ");
        assert_eq!(FINDING_REJECTED_PREFIX, "Rejected — ");
        assert_eq!(FINDING_REROUTED_PREFIX, "Rerouted — ");
    }

    #[test]
    fn brief_path_is_inside_the_repo_gitignored_tmp_dir() {
        let repo = Path::new("/checkout/repo");

        let first = brief_path(repo, "ab-review.4", 2);
        let repeated = brief_path(repo, "ab-review.4", 2);

        assert_eq!(first, repeated);
        assert!(first.starts_with(repo.join("target/abacus-tmp/reviews")));
        assert_eq!(
            first.file_name().and_then(|name| name.to_str()),
            Some("rev-ab-review-4-c2.md")
        );
    }

    #[test]
    fn reviewer_name_is_deterministic_safe_and_distinct_across_cycles() {
        let bead = "9/UNSAFE/abcdefghijklmnopqrstuvwxyz0123456789";
        let cycle_one = reviewer_name(bead, 1);
        let repeated = reviewer_name(bead, 1);
        let cycle_two = reviewer_name(bead, 2);

        assert_eq!(cycle_one, repeated);
        assert_ne!(cycle_one, cycle_two);
        for name in [&cycle_one, &cycle_two] {
            assert!(
                name.len() <= 32,
                "reviewer name exceeded Herdr's limit: {name}"
            );
            assert!(name.starts_with("rev-"));
            assert!(name.chars().all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, '-' | '_')
            }));
        }
        assert!(cycle_one.ends_with("-c1"));
        assert!(cycle_two.ends_with("-c2"));
    }
}

//! `abacus run` — single-pass dispatch: read the ready backlog, open a lane,
//! start a codex worker in it, send the dispatch prompt, wait for settle.
//! Everything stateful is shelled out to `br` and `herdr`; records,
//! acceptance, and evidence chains are deliberately absent (SHIFT-REPORT
//! 2026-08-13 §3).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(test)]
use abacus::BeadOutcome;
use abacus::land::{
    Candidate, CompositionResult, DecisionInput, Eligibility, LandDecision, LocalLeg, QueueState,
    ValidationFailure, admission_red_park_body, decide, dequeue_park_body, enumerate_candidates,
    parse_eligibility, parse_enqueue_result, parse_queue_state,
};
use abacus::lane::{
    LaneState, LaneStateInputs, MorningReport, PullRequestProbe, PullRequestState, capture,
    derive_lane_state, lane_open, lane_prompt, lane_reap, lane_reap_blocked, lane_settle,
    probe_bead_outcome,
};
#[cfg(test)]
use abacus::lane::{retry_never_engaged_once, retry_probe_once};
use abacus::review::{
    AdjudicationVerdict, BLOCKED_COMMENT_TOKEN, CommitStatusState, PostedReviewStatus,
    ReviewComment, ReviewCommentFacts, commit_status_request, launch_reviewer,
    parse_combined_status, parse_review_bead, review_comment_facts, reviewer_name,
};
use abacus::{
    format_lane_duration, parse_ready, parse_worktree_created, sanitize_agent_name, select_bead,
    version_string,
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
            match cmd_run(&repo) {
                Ok(0) => {}
                Ok(code) => exit(code),
                Err(e) => {
                    eprintln!("abacus run: {e}");
                    exit(1);
                }
            }
        }
        Some("drain") => {
            let repo = args.get(1).map(PathBuf::from).unwrap_or_else(|| ".".into());
            if let Err(e) = cmd_drain(&repo) {
                eprintln!("abacus drain: {e}");
                exit(1);
            }
        }
        Some("land") => {
            let (repo, once) = match parse_land_args(&args[1..]) {
                Ok(parsed) => parsed,
                Err(error) => {
                    eprintln!("abacus land: {error}");
                    print_usage();
                    exit(2);
                }
            };
            if let Err(e) = cmd_land(&repo, once) {
                eprintln!("abacus land: {e}");
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
    "usage: abacus run [repo-path]\n       abacus drain [repo-path]\n       abacus land [repo-path] [--once]\n       abacus merge-jsonl <ours> <base> <theirs>"
}

fn print_usage() {
    eprintln!("{}", usage());
}

fn parse_land_args(args: &[String]) -> Result<(PathBuf, bool), String> {
    let mut repo = None;
    let mut once = false;
    for arg in args {
        if arg == "--once" {
            once = true;
        } else if arg.starts_with('-') {
            return Err(format!("unknown option {arg:?}"));
        } else if repo.replace(PathBuf::from(arg)).is_some() {
            return Err("expected at most one repository path".into());
        }
    }
    Ok((repo.unwrap_or_else(|| ".".into()), once))
}

#[derive(serde::Deserialize)]
struct RepoView {
    #[serde(rename = "nameWithOwner")]
    name_with_owner: String,
    #[serde(rename = "defaultBranchRef")]
    default_branch_ref: DefaultBranchRef,
}

#[derive(serde::Deserialize)]
struct DefaultBranchRef {
    name: String,
}

fn land_repo_view(repo: &Path) -> Result<RepoView, String> {
    let output = capture(
        "gh",
        &["repo", "view", "--json", "nameWithOwner,defaultBranchRef"],
        Some(repo),
    )?;
    let view: RepoView = serde_json::from_str(&output)
        .map_err(|e| format!("unparseable `gh repo view` output: {e}"))?;
    if view.name_with_owner.trim().is_empty() || view.default_branch_ref.name.trim().is_empty() {
        return Err("`gh repo view` returned an empty repository or default branch".into());
    }
    Ok(view)
}

fn cmd_land(repo: &Path, once: bool) -> Result<(), String> {
    let repo = resolve_repo(repo)?;
    let view = land_repo_view(&repo)?;
    let ruleset_path = format!("repos/{}/rulesets", view.name_with_owner);
    let rulesets = capture("gh", &["api", &ruleset_path], Some(&repo))?;
    if let Eligibility::Ineligible { reason } = parse_eligibility(&rulesets)? {
        return Err(format!("repository is ineligible: {reason}"));
    }

    loop {
        land_cycle(&repo, &view, &mut land_poll_delay)?;
        if once {
            return Ok(());
        }
        land_poll_delay();
    }
}

fn land_poll_delay() {
    let millis = std::env::var("ABACUS_LAND_POLL_MILLIS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5_000);
    if millis > 0 {
        std::thread::sleep(Duration::from_millis(millis));
    }
}

fn land_cycle<Delay>(repo: &Path, view: &RepoView, delay: &mut Delay) -> Result<(), String>
where
    Delay: FnMut(),
{
    capture("git", &["fetch", "origin"], Some(repo))?;
    let open_prs = capture(
        "gh",
        &["pr", "list", "--state", "open", "--json", "headRefName"],
        Some(repo),
    )?;
    let closed_beads = capture("br", &["list", "--json"], Some(repo))?;
    let candidates = enumerate_candidates(&open_prs, &closed_beads)?;

    for candidate in candidates {
        land_candidate(repo, view, &candidate, delay)?;
    }
    Ok(())
}

fn land_candidate<Delay>(
    repo: &Path,
    view: &RepoView,
    candidate: &Candidate,
    delay: &mut Delay,
) -> Result<(), String>
where
    Delay: FnMut(),
{
    match observe_queue_state(repo, view, &candidate.branch)? {
        QueueState::Merged | QueueState::Queued => return Ok(()),
        QueueState::Dequeued(reason) => {
            let admitted_head_sha = candidate_head_sha(repo, candidate)?;
            return resolve_once(repo, view, candidate, &admitted_head_sha, &reason, delay);
        }
        QueueState::Absent => {}
    }

    let admission = admit_candidate(repo, &view.default_branch_ref.name, candidate)?;
    let decision = decide(DecisionInput::Admission {
        composition: admission.composition,
        local_leg: admission.local_leg.clone(),
        admitted_head_sha: admission.admitted_head_sha.clone(),
    });

    match decision {
        LandDecision::Enqueue { .. } => {
            enqueue_candidate(repo, &candidate.branch)?;
            watch_enqueued(
                repo,
                view,
                candidate,
                &admission.admitted_head_sha,
                0,
                delay,
            )
        }
        LandDecision::Park => {
            let LocalLeg::Fail(failure) = &admission.local_leg else {
                return Err(format!(
                    "policy parked {} without local failure evidence",
                    candidate.branch
                ));
            };
            let body =
                admission_red_park_body(&candidate.bead_id, &admission.admitted_head_sha, failure);
            comment_on_candidate(repo, &candidate.branch, &body)
        }
        LandDecision::Resolve => {
            let reason = format!(
                "admission composition conflict with origin/{}",
                view.default_branch_ref.name
            );
            resolve_once(
                repo,
                view,
                candidate,
                &admission.admitted_head_sha,
                &reason,
                delay,
            )
        }
    }
}

fn enqueue_candidate(repo: &Path, branch: &str) -> Result<(), String> {
    let result = capture_status("gh", &["pr", "merge", branch], Some(repo))?;
    parse_enqueue_result(result)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn comment_on_candidate(repo: &Path, branch: &str, body: &str) -> Result<(), String> {
    capture("gh", &["pr", "comment", branch, "--body", body], Some(repo)).map(|_| ())
}

fn watch_enqueued<Delay>(
    repo: &Path,
    view: &RepoView,
    candidate: &Candidate,
    admitted_head_sha: &str,
    completed_attempts: u8,
    delay: &mut Delay,
) -> Result<(), String>
where
    Delay: FnMut(),
{
    loop {
        match observe_queue_state(repo, view, &candidate.branch)? {
            QueueState::Merged => return Ok(()),
            QueueState::Queued | QueueState::Absent => delay(),
            QueueState::Dequeued(reason) => {
                return match decide(DecisionInput::Dequeued {
                    attempts: completed_attempts,
                }) {
                    LandDecision::Resolve => {
                        resolve_once(repo, view, candidate, admitted_head_sha, &reason, delay)
                    }
                    LandDecision::Park => {
                        park_resolution(repo, candidate, admitted_head_sha, &reason, None)
                    }
                    LandDecision::Enqueue { .. } => {
                        Err("dequeue policy unexpectedly requested enqueue".into())
                    }
                };
            }
        }
    }
}

fn candidate_head_sha(repo: &Path, candidate: &Candidate) -> Result<String, String> {
    let branch_ref = format!("origin/{}", candidate.branch);
    let sha = capture("git", &["rev-parse", &branch_ref], Some(repo))?;
    let sha = sha.trim();
    if sha.is_empty() {
        return Err(format!("empty head SHA for {}", candidate.branch));
    }
    Ok(sha.into())
}

fn resolution_prompt(candidate: &Candidate, default_branch: &str, reason: &str) -> String {
    format!(
        "You are the merge-queue resolution lane for bead {bead}, attempt 1 of 1, on the existing \
         PR branch {branch}. Resolve this exception against the freshly fetched default branch \
         {default_branch}: {reason}. This bead stays closed; do not run any br write command. \
         Reconfirm the requested codex provider identity at every execution. Do not rebase, \
         force-push, merge the PR, or delete its branch. Run the full test suite, clippy with \
         warnings denied, and fmt check. Commit the resolution and push it from this lane to \
         origin {branch}; abacus itself never pushes. If the exception cannot be resolved, leave \
         the PR open and report the evidence.",
        bead = candidate.bead_id,
        branch = candidate.branch,
    )
}

fn dispatch_resolution(
    repo: &Path,
    candidate: &Candidate,
    default_branch: &str,
    reason: &str,
) -> Result<(), String> {
    let repo_arg = repo.to_string_lossy().into_owned();
    let opened = capture(
        "herdr",
        &[
            "worktree",
            "open",
            "--cwd",
            &repo_arg,
            "--branch",
            &candidate.branch,
            "--label",
            &candidate.bead_id,
            "--no-focus",
        ],
        None,
    )?;
    let lane = parse_worktree_created(&opened)?;
    let agent_name = sanitize_agent_name(&format!("r-{}", candidate.bead_id));
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
    let prompt = resolution_prompt(candidate, default_branch, reason);
    capture(
        "herdr",
        &["agent", "prompt", &agent_name, &prompt, "--wait"],
        None,
    )?;
    Ok(())
}

fn resolve_once<Delay>(
    repo: &Path,
    view: &RepoView,
    candidate: &Candidate,
    admitted_head_sha: &str,
    reason: &str,
    delay: &mut Delay,
) -> Result<(), String>
where
    Delay: FnMut(),
{
    if let Err(error) = dispatch_resolution(repo, candidate, &view.default_branch_ref.name, reason)
    {
        return park_resolution(repo, candidate, admitted_head_sha, reason, Some(&error));
    }

    capture("git", &["fetch", "origin"], Some(repo))?;
    let readmission = match admit_candidate(repo, &view.default_branch_ref.name, candidate) {
        Ok(readmission) => readmission,
        Err(error) => {
            return park_resolution(repo, candidate, admitted_head_sha, reason, Some(&error));
        }
    };
    let decision = decide(DecisionInput::Readmission {
        composition: readmission.composition,
        local_leg: readmission.local_leg.clone(),
        admitted_head_sha: readmission.admitted_head_sha.clone(),
        attempts: 1,
    });
    match decision {
        LandDecision::Enqueue { .. } => {
            enqueue_candidate(repo, &candidate.branch)?;
            watch_enqueued(
                repo,
                view,
                candidate,
                &readmission.admitted_head_sha,
                1,
                delay,
            )
        }
        LandDecision::Park => {
            let detail = match &readmission.local_leg {
                LocalLeg::Fail(failure) => Some(format!(
                    "readmission failed in {}: {}",
                    failure.tool,
                    failure.stderr.trim()
                )),
                _ if readmission.composition == CompositionResult::Conflict => {
                    Some("readmission still has a composition conflict".into())
                }
                _ => None,
            };
            park_resolution(
                repo,
                candidate,
                admitted_head_sha,
                reason,
                detail.as_deref(),
            )
        }
        LandDecision::Resolve => Err("readmission requested a second resolution attempt".into()),
    }
}

fn park_resolution(
    repo: &Path,
    candidate: &Candidate,
    admitted_head_sha: &str,
    reason: &str,
    detail: Option<&str>,
) -> Result<(), String> {
    let mut body = dequeue_park_body(&candidate.bead_id, admitted_head_sha, reason)?;
    if let Some(detail) = detail.filter(|detail| !detail.trim().is_empty()) {
        body.push_str("\n\nResolution evidence: ");
        body.push_str(detail.trim());
    }
    comment_on_candidate(repo, &candidate.branch, &body)
}

const QUEUE_QUERY: &str = r#"query($owner:String!,$name:String!,$number:Int!,$branch:String!){repository(owner:$owner,name:$name){ref(qualifiedName:$branch){name} pullRequest(number:$number){state merged isInMergeQueue autoMergeRequest{enabledAt} mergeQueueEntry{id} timelineItems(last:1,itemTypes:[REMOVED_FROM_MERGE_QUEUE_EVENT]){nodes{... on RemovedFromMergeQueueEvent{reason}}}}}}"#;

fn observe_queue_state(repo: &Path, view: &RepoView, branch: &str) -> Result<QueueState, String> {
    let number = capture(
        "gh",
        &["pr", "view", branch, "--json", "number", "--jq", ".number"],
        Some(repo),
    )?;
    let number = number.trim();
    if number.is_empty() || !number.chars().all(|character| character.is_ascii_digit()) {
        return Err(format!("unexpected PR number for {branch}: {number:?}"));
    }
    let (owner, name) = view.name_with_owner.split_once('/').ok_or_else(|| {
        format!(
            "unexpected GitHub repository name {:?}",
            view.name_with_owner
        )
    })?;
    let owner_arg = format!("owner={owner}");
    let name_arg = format!("name={name}");
    let number_arg = format!("number={number}");
    let branch_arg = format!("branch={branch}");
    let query_arg = format!("query={QUEUE_QUERY}");
    let output = capture(
        "gh",
        &[
            "api",
            "graphql",
            "-f",
            &query_arg,
            "-F",
            &owner_arg,
            "-F",
            &name_arg,
            "-F",
            &number_arg,
            "-F",
            &branch_arg,
        ],
        Some(repo),
    )?;
    parse_queue_state(&output)
}

#[derive(Debug)]
struct Admission {
    composition: CompositionResult,
    local_leg: LocalLeg,
    admitted_head_sha: String,
}

static ADMISSION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn admission_worktree_path() -> PathBuf {
    let sequence = ADMISSION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "abacus-land-admission-{}-{started_at}-{sequence}",
        std::process::id(),
    ))
}

fn admit_candidate(
    repo: &Path,
    default_branch: &str,
    candidate: &Candidate,
) -> Result<Admission, String> {
    let branch_ref = format!("origin/{}", candidate.branch);
    let admitted_head_sha = candidate_head_sha(repo, candidate)?;
    let worktree = admission_worktree_path();
    if worktree.exists() {
        return Err(format!(
            "admission worktree path already exists: {}",
            worktree.display()
        ));
    }
    let worktree_arg = worktree.to_string_lossy().into_owned();
    capture(
        "git",
        &["worktree", "add", "--detach", &worktree_arg, &branch_ref],
        Some(repo),
    )?;

    let result = admit_in_worktree(&worktree, default_branch, &admitted_head_sha);
    let cleanup = capture("git", &["worktree", "remove", &worktree_arg], Some(repo));
    match (result, cleanup) {
        (Ok(admission), Ok(_)) => Ok(admission),
        (Err(error), Ok(_)) => Err(error),
        (Ok(_), Err(cleanup_error)) => Err(format!(
            "admission succeeded but worktree cleanup failed: {cleanup_error}"
        )),
        (Err(error), Err(cleanup_error)) => Err(format!(
            "{error}; admission worktree cleanup also failed: {cleanup_error}"
        )),
    }
}

fn admit_in_worktree(
    worktree: &Path,
    default_branch: &str,
    admitted_head_sha: &str,
) -> Result<Admission, String> {
    let default_ref = format!("origin/{default_branch}");
    let (merge_code, _, merge_stderr) =
        capture_status("git", &["merge", &default_ref], Some(worktree))?;
    if merge_code != 0 {
        let conflicts = capture(
            "git",
            &["diff", "--name-only", "--diff-filter=U"],
            Some(worktree),
        )?;
        if conflicts.trim().is_empty() {
            let _ = capture_status("git", &["merge", "--abort"], Some(worktree));
            return Err(format!(
                "composition merge failed without conflicts: {}",
                merge_stderr.trim()
            ));
        }
        capture("git", &["merge", "--abort"], Some(worktree))?;
        return Ok(Admission {
            composition: CompositionResult::Conflict,
            local_leg: LocalLeg::NotRun,
            admitted_head_sha: admitted_head_sha.into(),
        });
    }

    let validations: [(&str, &[&str]); 3] = [
        ("cargo test", &["test"]),
        (
            "cargo clippy",
            &[
                "clippy",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        ),
        ("cargo fmt", &["fmt", "--check"]),
    ];
    for (tool, args) in validations {
        let (code, _, stderr) = capture_status("cargo", args, Some(worktree))?;
        if code != 0 {
            return Ok(Admission {
                composition: CompositionResult::Clean,
                local_leg: LocalLeg::Fail(ValidationFailure {
                    tool: tool.into(),
                    stderr,
                }),
                admitted_head_sha: admitted_head_sha.into(),
            });
        }
    }

    Ok(Admission {
        composition: CompositionResult::Clean,
        local_leg: LocalLeg::Pass,
        admitted_head_sha: admitted_head_sha.into(),
    })
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

enum DispatchCycle {
    Empty,
    Settled(SettledLane),
    ClaimLost(String),
}

struct SettledLane {
    bead_id: String,
    lane: abacus::Lane,
    lane_available: bool,
    outcome: abacus::BeadOutcome,
    elapsed_secs: u64,
}

fn resolve_repo(repo: &Path) -> Result<PathBuf, String> {
    repo.canonicalize()
        .map_err(|e| format!("cannot resolve repo path {}: {e}", repo.display()))
}

fn cmd_run(repo: &Path) -> Result<i32, String> {
    let repo = resolve_repo(repo)?;
    let repo_str = repo.to_string_lossy().into_owned();
    match dispatch_cycle(&repo, &repo_str, &BTreeSet::new(), false)? {
        DispatchCycle::Empty => {
            println!("no ready beads in {repo_str}; nothing to dispatch");
            Ok(0)
        }
        DispatchCycle::Settled(settled) => {
            let observation = derive_settled_lane_state(&repo, &settled, false)?;
            let mut launched_reviewers = BTreeSet::new();
            reconcile_review_lifecycle(
                &repo,
                &settled,
                &observation,
                &[],
                &mut launched_reviewers,
            )?;
            match observation.state {
                None => {
                    lane_reap(settled.outcome, &settled.lane)?;
                    println!(
                        "bead {} is closed; worker completed in {}",
                        settled.bead_id,
                        format_lane_duration(settled.elapsed_secs)
                    );
                    Ok(0)
                }
                Some(LaneState::AwaitingReview) => {
                    println!(
                        "bead {} lane is awaiting-review after {}; leaving it warm",
                        settled.bead_id,
                        format_lane_duration(settled.elapsed_secs)
                    );
                    Ok(0)
                }
                Some(LaneState::Merged) => {
                    lane_reap(abacus::BeadOutcome::Completed, &settled.lane)?;
                    println!(
                        "bead {} lane is merged; reaped after {}",
                        settled.bead_id,
                        format_lane_duration(settled.elapsed_secs)
                    );
                    Ok(0)
                }
                Some(LaneState::Blocked) => {
                    lane_reap_blocked(&settled.lane)?;
                    eprintln!(
                        "bead {} is in_progress; worker reported {} after {}",
                        settled.bead_id,
                        BLOCKED_COMMENT_TOKEN,
                        format_lane_duration(settled.elapsed_secs)
                    );
                    Ok(3)
                }
                Some(LaneState::Stalled) => {
                    match settled.outcome {
                        abacus::BeadOutcome::NeverEngaged => eprintln!(
                            "bead {} is open; lane is stalled because the worker never engaged after {}",
                            settled.bead_id,
                            format_lane_duration(settled.elapsed_secs)
                        ),
                        _ => eprintln!(
                            "bead {} is in_progress; lane is stalled after worker settled before completing ({})",
                            settled.bead_id,
                            format_lane_duration(settled.elapsed_secs)
                        ),
                    }
                    Ok(3)
                }
                Some(state @ (LaneState::Authoring | LaneState::ReworkRequested)) => {
                    eprintln!(
                        "bead {} settled into parked lane state {state:?} after {}",
                        settled.bead_id,
                        format_lane_duration(settled.elapsed_secs)
                    );
                    Ok(3)
                }
            }
        }
        DispatchCycle::ClaimLost(_) => unreachable!("run treats claim failures as errors"),
    }
}

fn cmd_drain(repo: &Path) -> Result<(), String> {
    let repo = resolve_repo(repo)?;
    let repo_str = repo.to_string_lossy().into_owned();
    let mut lost_claims = BTreeSet::new();
    let mut report = MorningReport::default();
    let mut reported_states = BTreeSet::new();
    let mut launched_reviewers = BTreeSet::new();
    // Invocation-local only: restart must re-derive durable lane facts.
    // Within a run, Merged and closed/no-PR results are terminal; live lanes
    // stay eligible so a later sweep can observe their transitions.
    let mut absorbed_terminal_beads = BTreeSet::new();

    loop {
        if sweep_live_lanes(
            &repo,
            &mut report,
            &mut reported_states,
            &mut absorbed_terminal_beads,
            &mut launched_reviewers,
        )? {
            std::thread::sleep(Duration::from_secs(2));
            continue;
        }
        match dispatch_cycle(&repo, &repo_str, &lost_claims, true)? {
            DispatchCycle::Empty => {
                println!("no ready beads in {repo_str}; nothing to dispatch");
                let rendered = report.render();
                if !rendered.is_empty() {
                    println!("morning report:\n{rendered}");
                }
                return Ok(());
            }
            DispatchCycle::Settled(settled) => {
                record_drain_settle(
                    &repo,
                    settled,
                    false,
                    &[],
                    &mut launched_reviewers,
                    &mut report,
                    &mut reported_states,
                )?;
            }
            DispatchCycle::ClaimLost(bead_id) => {
                lost_claims.insert(bead_id);
            }
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct AgentListEnvelope {
    result: AgentListResult,
}

#[derive(Debug, serde::Deserialize)]
struct AgentListResult {
    #[serde(default)]
    agents: Vec<AgentView>,
}

#[derive(Debug, serde::Deserialize)]
struct AgentView {
    #[serde(default)]
    name: Option<String>,
    agent_status: String,
    cwd: String,
    workspace_id: String,
    pane_id: String,
}

#[derive(Debug, serde::Deserialize)]
struct BeadListEnvelope {
    issues: Vec<ListedLaneBead>,
}

#[derive(Debug, serde::Deserialize)]
struct ListedLaneBead {
    id: String,
    status: String,
}

fn parse_agent_list(json: &str) -> Result<Vec<AgentView>, String> {
    if json.trim().is_empty() {
        return Ok(Vec::new());
    }
    let envelope: AgentListEnvelope = serde_json::from_str(json)
        .map_err(|error| format!("unparseable `herdr agent list` output: {error}"))?;
    Ok(envelope.result.agents)
}

fn parse_lane_beads(json: &str) -> Result<Vec<ListedLaneBead>, String> {
    let envelope: BeadListEnvelope = serde_json::from_str(json)
        .map_err(|error| format!("unparseable `br list --json` output: {error}"))?;
    Ok(envelope.issues)
}

fn agent_belongs_to_repo(agent: &AgentView, repo: &Path) -> bool {
    let Some(repo_name) = repo.file_name() else {
        return false;
    };
    Path::new(&agent.cwd) == repo
        || Path::new(&agent.cwd)
            .components()
            .any(|component| component.as_os_str() == repo_name)
}

/// Reconstruct warm lanes from durable bead state plus Herdr's deterministic
/// agent names. Returning true means an author is still active, so the serial
/// drain must sweep again rather than dispatching a second worker.
fn sweep_live_lanes(
    repo: &Path,
    report: &mut MorningReport,
    reported_states: &mut BTreeSet<(String, String)>,
    absorbed_terminal_beads: &mut BTreeSet<String>,
    launched_reviewers: &mut BTreeSet<(String, u32)>,
) -> Result<bool, String> {
    let agents = parse_agent_list(&capture("herdr", &["agent", "list"], None)?)?;
    let agents: Vec<_> = agents
        .into_iter()
        .filter(|agent| agent.name.is_some() && agent_belongs_to_repo(agent, repo))
        .collect();
    let beads = parse_lane_beads(&capture("br", &["list", "--json"], Some(repo))?)?;
    let mut authoring = false;
    for bead in beads {
        if absorbed_terminal_beads.contains(&bead.id)
            || (bead.status != "in_progress" && bead.status != "closed")
        {
            continue;
        }
        let bead_is_closed = bead.status == "closed";
        let agent_name = sanitize_agent_name(&bead.id);
        let agent = agents
            .iter()
            .find(|agent| agent.name.as_deref() == Some(agent_name.as_str()));
        let outcome = probe_bead_outcome(repo, &bead.id)?;
        let worker_active = agent.is_some_and(|agent| agent.agent_status == "working");
        let lane = agent.map_or_else(
            || abacus::Lane {
                workspace_id: String::new(),
                pane_id: String::new(),
                checkout_path: repo.to_string_lossy().into_owned(),
                branch: format!("lane/{}", bead.id),
            },
            |agent| abacus::Lane {
                workspace_id: agent.workspace_id.clone(),
                pane_id: agent.pane_id.clone(),
                checkout_path: agent.cwd.clone(),
                branch: format!("lane/{}", bead.id),
            },
        );
        let lane_available = agent.is_some();
        let bead_id = bead.id;
        let state = record_drain_settle(
            repo,
            SettledLane {
                bead_id: bead_id.clone(),
                lane,
                lane_available,
                outcome,
                elapsed_secs: 0,
            },
            worker_active,
            &agents,
            launched_reviewers,
            report,
            reported_states,
        )?;
        if state == Some(LaneState::Merged) || (bead_is_closed && state.is_none()) {
            absorbed_terminal_beads.insert(bead_id);
        }
        authoring |= state == Some(LaneState::Authoring);
    }
    Ok(authoring)
}

#[derive(serde::Deserialize)]
struct PullRequestView {
    state: String,
    #[serde(rename = "mergedAt")]
    merged_at: Option<String>,
    #[serde(rename = "headRefOid", default)]
    head_ref_oid: Option<String>,
    #[serde(default)]
    number: Option<u64>,
    #[serde(default)]
    comments: Vec<PullRequestComment>,
}

#[derive(serde::Deserialize)]
struct PullRequestComment {
    body: String,
    #[serde(default)]
    author: Option<PullRequestCommentAuthor>,
    #[serde(rename = "authorAssociation", default)]
    author_association: String,
}

#[derive(serde::Deserialize)]
struct PullRequestCommentAuthor {
    login: String,
}

struct PullRequestObservation {
    probe: PullRequestProbe,
    number: Option<u64>,
    review_facts: ReviewCommentFacts,
}

struct SettledLaneObservation {
    state: Option<LaneState>,
    pull_request: Option<PullRequestObservation>,
}

fn parse_pull_request_probe(json: &str) -> Result<PullRequestObservation, String> {
    let view: PullRequestView = serde_json::from_str(json)
        .map_err(|error| format!("unparseable `gh pr view` output: {error}"))?;
    let state = if view.merged_at.is_some() || view.state == "MERGED" {
        PullRequestState::Merged
    } else {
        match view.state.as_str() {
            "OPEN" => PullRequestState::Open,
            "CLOSED" => PullRequestState::Closed,
            other => return Err(format!("unsupported pull request state {other:?}")),
        }
    };
    let comments: Vec<_> = view
        .comments
        .iter()
        .map(|comment| ReviewComment {
            body: &comment.body,
            author_login: comment
                .author
                .as_ref()
                .map_or("", |author| author.login.as_str()),
            author_association: &comment.author_association,
        })
        .collect();
    Ok(PullRequestObservation {
        probe: PullRequestProbe {
            state,
            head_sha: view.head_ref_oid,
        },
        number: view.number,
        review_facts: review_comment_facts(&comments)?,
    })
}

fn is_no_pull_request_error(stderr: &str) -> bool {
    stderr.contains("no pull requests found")
        || stderr.contains("Could not resolve to a PullRequest")
        || stderr.contains("no open pull requests")
        || stderr.contains("no git remotes found")
        || stderr.contains(
            "none of the git remotes configured for this repository point to a known GitHub host",
        )
}

fn probe_pull_request(repo: &Path, branch: &str) -> Result<Option<PullRequestObservation>, String> {
    let (code, stdout, stderr) = capture_status(
        "gh",
        &[
            "pr",
            "view",
            branch,
            "--json",
            "state,mergedAt,headRefOid,number,comments",
        ],
        Some(repo),
    )?;
    if code == 0 {
        parse_pull_request_probe(&stdout).map(Some)
    } else if is_no_pull_request_error(&stderr) {
        Ok(None)
    } else {
        Err(format!(
            "`gh pr view {branch} --json state,mergedAt,headRefOid,number,comments` failed ({code}): {}",
            stderr.trim()
        ))
    }
}

fn record_drain_settle(
    repo: &Path,
    settled: SettledLane,
    worker_active: bool,
    agents: &[AgentView],
    launched_reviewers: &mut BTreeSet<(String, u32)>,
    report: &mut MorningReport,
    reported_states: &mut BTreeSet<(String, String)>,
) -> Result<Option<LaneState>, String> {
    let observation = derive_settled_lane_state(repo, &settled, worker_active)?;
    reconcile_review_lifecycle(repo, &settled, &observation, agents, launched_reviewers)?;
    let state = observation.state;
    if state.is_none() {
        let key = (settled.bead_id.clone(), "completed".to_owned());
        if reported_states.insert(key) {
            if settled.lane_available {
                lane_reap(settled.outcome, &settled.lane)?;
            }
            report.record_completed(&settled.bead_id, settled.elapsed_secs);
        }
        return Ok(None);
    }
    let state = state.expect("the completed-without-PR case returned above");
    if state == LaneState::Authoring {
        return Ok(Some(state));
    }
    let key = (settled.bead_id.clone(), format!("{state:?}"));
    if !reported_states.insert(key) {
        return Ok(Some(state));
    }
    match state {
        LaneState::Blocked => {
            if settled.lane_available {
                lane_reap_blocked(&settled.lane)?;
            }
        }
        LaneState::Merged => {
            if settled.lane_available {
                lane_reap(abacus::BeadOutcome::Completed, &settled.lane)?;
            }
        }
        LaneState::AwaitingReview => {}
        LaneState::Authoring | LaneState::ReworkRequested | LaneState::Stalled => {}
    }
    report.record_state(state, &settled.bead_id, settled.elapsed_secs);
    Ok(Some(state))
}

fn probe_pull_request_number(repo: &Path, branch: &str) -> Result<u64, String> {
    let number = capture(
        "gh",
        &["pr", "view", branch, "--json", "number", "--jq", ".number"],
        Some(repo),
    )?;
    number.trim().parse::<u64>().map_err(|error| {
        format!(
            "unexpected PR number for {branch}: {:?}: {error}",
            number.trim()
        )
    })
}

fn launch_awaiting_reviewer(
    repo: &Path,
    settled: &SettledLane,
    pr_number: Option<u64>,
    cycle: u32,
) -> Result<(), String> {
    let bead_json = capture("br", &["show", &settled.bead_id, "--json"], Some(repo))?;
    let review_bead = parse_review_bead(&bead_json)?;
    let pr_number = match pr_number {
        Some(number) => number,
        None => probe_pull_request_number(repo, &settled.lane.branch)?,
    };
    let brief = launch_reviewer(repo, &settled.bead_id, &review_bead, pr_number, cycle)?;
    println!(
        "adversarial reviewer launched for {} cycle {} with brief {}",
        settled.bead_id,
        cycle,
        brief.display()
    );
    Ok(())
}

fn post_commit_status(repo: &Path, head_sha: &str, state: CommitStatusState) -> Result<(), String> {
    let request = commit_status_request(head_sha, state);
    let state_field = format!("state={}", request.state.as_str());
    let context_field = format!("context={}", request.context);
    capture(
        "gh",
        &[
            "api",
            "--method",
            "POST",
            &request.endpoint,
            "-f",
            &state_field,
            "-f",
            &context_field,
        ],
        Some(repo),
    )?;
    Ok(())
}

fn reconcile_commit_status(
    repo: &Path,
    state: LaneState,
    pull_request: &PullRequestObservation,
) -> Result<(), String> {
    let Some(head_sha) = pull_request.probe.head_sha.as_deref() else {
        return Ok(());
    };
    let accepted_current_head = pull_request
        .review_facts
        .latest_adjudication
        .as_ref()
        .is_some_and(|adjudication| {
            adjudication.verdict == AdjudicationVerdict::Accepted
                && adjudication.adjudicated_head == head_sha
        });
    let desired = if accepted_current_head {
        Some(CommitStatusState::Success)
    } else if state == LaneState::AwaitingReview {
        Some(CommitStatusState::Pending)
    } else {
        None
    };
    let Some(desired) = desired else {
        return Ok(());
    };

    let endpoint = format!("repos/{{owner}}/{{repo}}/commits/{head_sha}/status");
    let combined = capture("gh", &["api", &endpoint], Some(repo))?;
    let posted = parse_combined_status(&combined)?;
    let already_posted = matches!(
        (desired, posted),
        (CommitStatusState::Pending, PostedReviewStatus::Pending)
            | (CommitStatusState::Success, PostedReviewStatus::Success)
    );
    if !already_posted {
        post_commit_status(repo, head_sha, desired)?;
    }
    Ok(())
}

fn reap_reviewers_with_verdicts(
    bead_id: &str,
    facts: &ReviewCommentFacts,
    agents: &[AgentView],
) -> Result<(), String> {
    for cycle in &facts.verdict_cycles {
        let name = reviewer_name(bead_id, *cycle);
        if let Some(agent) = agents
            .iter()
            .find(|agent| agent.name.as_deref() == Some(name.as_str()))
        {
            capture("herdr", &["workspace", "close", &agent.workspace_id], None)?;
            println!(
                "adversarial reviewer reaped for {bead_id} cycle {cycle}: workspace {}",
                agent.workspace_id
            );
        }
    }
    Ok(())
}

fn reconcile_review_lifecycle(
    repo: &Path,
    settled: &SettledLane,
    observation: &SettledLaneObservation,
    agents: &[AgentView],
    launched_reviewers: &mut BTreeSet<(String, u32)>,
) -> Result<(), String> {
    let Some(pull_request) = observation.pull_request.as_ref() else {
        return Ok(());
    };
    reap_reviewers_with_verdicts(&settled.bead_id, &pull_request.review_facts, agents)?;
    let Some(state) = observation.state else {
        return Ok(());
    };
    reconcile_commit_status(repo, state, pull_request)?;

    if state != LaneState::AwaitingReview {
        return Ok(());
    }
    let accepted_current_head = pull_request
        .review_facts
        .latest_adjudication
        .as_ref()
        .is_some_and(|adjudication| {
            adjudication.verdict == AdjudicationVerdict::Accepted
                && pull_request.probe.head_sha.as_deref()
                    == Some(adjudication.adjudicated_head.as_str())
        });
    if accepted_current_head {
        return Ok(());
    }
    let cycle = pull_request
        .review_facts
        .latest_adjudication
        .as_ref()
        .map_or(1, |adjudication| adjudication.cycle + 1);
    if pull_request.review_facts.verdict_cycles.contains(&cycle) {
        return Ok(());
    }
    let live_reviewer_name = reviewer_name(&settled.bead_id, cycle);
    if agents
        .iter()
        .any(|agent| agent.name.as_deref() == Some(live_reviewer_name.as_str()))
    {
        return Ok(());
    }
    if !launched_reviewers.insert((settled.bead_id.clone(), cycle)) {
        return Ok(());
    }
    launch_awaiting_reviewer(repo, settled, pull_request.number, cycle)
}

/// Derive the shared lane state for a settled worker. A completed bead with no
/// PR remains the legacy completed-and-reap case, which has no LaneState row.
fn derive_settled_lane_state(
    repo: &Path,
    settled: &SettledLane,
    worker_active: bool,
) -> Result<SettledLaneObservation, String> {
    let pull_request = if settled.outcome == abacus::BeadOutcome::Blocked {
        None
    } else {
        probe_pull_request(repo, &settled.lane.branch)?
    };
    if settled.outcome == abacus::BeadOutcome::Completed && pull_request.is_none() {
        return Ok(SettledLaneObservation {
            state: None,
            pull_request: None,
        });
    }
    let latest_adjudication = pull_request
        .as_ref()
        .and_then(|pull_request| pull_request.review_facts.latest_adjudication.as_ref())
        .map(|adjudication| abacus::lane::AdjudicationProbe {
            disposition: match adjudication.verdict {
                AdjudicationVerdict::Accepted => abacus::lane::AdjudicationDisposition::Accepted,
                AdjudicationVerdict::Rework => abacus::lane::AdjudicationDisposition::Rework,
            },
            adjudicated_head: adjudication.adjudicated_head.as_str(),
        });
    let state = derive_lane_state(LaneStateInputs {
        bead_outcome: settled.outcome,
        worker_active,
        pull_request: pull_request
            .as_ref()
            .map(|pull_request| &pull_request.probe),
        verdict_heading_count: pull_request.as_ref().map_or(0, |pull_request| {
            pull_request.review_facts.verdict_cycles.len()
        }),
        latest_adjudication,
    });
    Ok(SettledLaneObservation {
        state: Some(state),
        pull_request,
    })
}

fn parse_symbolic_default_branch(output: &str) -> Result<String, String> {
    let symbolic_ref = output.trim();
    symbolic_ref
        .strip_prefix("origin/")
        .filter(|branch| !branch.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("unexpected origin/HEAD symbolic ref: {symbolic_ref:?}"))
}

fn parse_advertised_default_branch(output: &str) -> Result<String, String> {
    for line in output.lines() {
        let mut fields = line.split_whitespace();
        if fields.next() != Some("ref:") {
            continue;
        }
        let Some(reference) = fields.next() else {
            continue;
        };
        if fields.next() != Some("HEAD") || fields.next().is_some() {
            continue;
        }
        if let Some(branch) = reference
            .strip_prefix("refs/heads/")
            .filter(|branch| !branch.is_empty())
        {
            return Ok(branch.to_owned());
        }
    }

    Err(format!(
        "unexpected advertised remote HEAD: {:?}",
        output.trim()
    ))
}

fn discover_default_branch(repo: &Path) -> Result<String, String> {
    let symbolic_attempt = capture(
        "git",
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
        Some(repo),
    )
    .and_then(|output| parse_symbolic_default_branch(&output));
    let symbolic_error = match symbolic_attempt {
        Ok(branch) => return Ok(branch),
        Err(error) => error,
    };

    capture(
        "git",
        &["ls-remote", "--symref", "origin", "HEAD"],
        Some(repo),
    )
    .and_then(|output| parse_advertised_default_branch(&output))
    .map_err(|advertised_error| {
        format!(
            "default branch discovery failed after both attempts: \
             `git symbolic-ref --short refs/remotes/origin/HEAD`: {symbolic_error}; \
             `git ls-remote --symref origin HEAD`: {advertised_error}"
        )
    })
}

fn dispatch_cycle(
    repo: &Path,
    repo_str: &str,
    lost_claims: &BTreeSet<String>,
    reselect_after_claim_failure: bool,
) -> Result<DispatchCycle, String> {
    let ready = capture("br", &["ready", "--json"], Some(repo))?;
    let beads = parse_ready(&ready)?;
    let claimable: Vec<_> = beads
        .into_iter()
        .filter(|bead| !lost_claims.contains(&bead.id))
        .collect();
    let Some(bead) = select_bead(&claimable).cloned() else {
        return Ok(DispatchCycle::Empty);
    };
    let default_branch = discover_default_branch(repo)?;
    if let Err(error) = capture("br", &["update", &bead.id, "--claim"], Some(repo)) {
        if reselect_after_claim_failure {
            eprintln!(
                "claim for {} failed; reselecting another ready bead: {error}",
                bead.id
            );
            return Ok(DispatchCycle::ClaimLost(bead.id.clone()));
        }
        return Err(error);
    }
    println!("selected {} — {}", bead.id, bead.title);

    let agent_name = sanitize_agent_name(&bead.id);
    let lane_started = Instant::now();
    let lane_result = (|| -> Result<(abacus::Lane, abacus::BeadOutcome), String> {
        let lane = lane_open(repo_str, &bead, &agent_name)?;
        let prompt = lane_prompt(&bead, &lane, &default_branch, &agent_name)?;
        let outcome = lane_settle(repo, &bead, &prompt)?;
        Ok((lane, outcome))
    })();

    let (lane, outcome) = lane_result.map_err(|error| {
        let duration = format_lane_duration(lane_started.elapsed().as_secs());
        format!("{error} after {duration}")
    })?;
    Ok(DispatchCycle::Settled(SettledLane {
        bead_id: bead.id,
        lane,
        lane_available: true,
        outcome,
        elapsed_secs: lane_started.elapsed().as_secs(),
    }))
}

/// Run a command and capture its exit code, stdout, and stderr without
/// treating a non-zero exit as an error.
fn capture_status(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
) -> Result<(i32, String, String), String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("failed to spawn {program}: {e}"))?;
    Ok((
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_live_agent_activity_without_treating_presence_as_working() {
        let json = r#"{"result":{"agents":[
            {"name":"ab-working","agent_status":"working","cwd":"/repo","workspace_id":"w1","pane_id":"w1:p1"},
            {"name":"ab-done","agent_status":"done","cwd":"/repo","workspace_id":"w2","pane_id":"w2:p1"},
            {"agent_status":"idle","cwd":"/repo","workspace_id":"w3","pane_id":"w3:p1"}
        ]}}"#;

        let agents = parse_agent_list(json).unwrap();
        assert_eq!(agents.len(), 3);
        assert_eq!(agents[0].name.as_deref(), Some("ab-working"));
        assert_eq!(agents[0].agent_status, "working");
        assert_eq!(agents[1].agent_status, "done");
        assert!(agents[2].name.is_none());
        assert!(parse_agent_list("").unwrap().is_empty());
    }

    #[test]
    fn pull_request_probe_distinguishes_open_merged_and_absent() {
        assert_eq!(
            parse_pull_request_probe(r#"{"state":"OPEN","mergedAt":null,"headRefOid":"head-1"}"#)
                .unwrap()
                .probe,
            PullRequestProbe {
                state: PullRequestState::Open,
                head_sha: Some("head-1".into()),
            }
        );
        assert_eq!(
            parse_pull_request_probe(
                r#"{"state":"CLOSED","mergedAt":"2026-08-19T10:00:00Z","headRefOid":"head-2"}"#
            )
            .unwrap()
            .probe
            .state,
            PullRequestState::Merged
        );
        assert!(is_no_pull_request_error(
            "no pull requests found for branch lane/ab-none"
        ));
        assert!(is_no_pull_request_error("no git remotes found"));
        assert!(is_no_pull_request_error(
            "none of the git remotes configured for this repository point to a known GitHub host"
        ));
        assert!(!is_no_pull_request_error("HTTP 502 from github.com"));
    }

    #[test]
    fn capture_status_preserves_non_zero_exit_code_and_output() {
        let (code, stdout, stderr) =
            capture_status("sh", &["-c", "printf out; printf err >&2; exit 8"], None).unwrap();

        assert_eq!(code, 8);
        assert_eq!(stdout, "out");
        assert_eq!(stderr, "err");
    }

    #[test]
    fn capture_non_zero_error_keeps_command_line_and_trimmed_stderr() {
        let command = "printf ignored; printf '  failure detail  \\n' >&2; exit 7";

        let error = capture("sh", &["-c", command], None).unwrap_err();

        assert!(error.contains(&format!("`sh -c {command}` failed")));
        assert!(error.ends_with(": failure detail"));
    }

    #[test]
    fn parses_the_advertised_remote_head_branch() {
        let output = "ref: refs/heads/release/next\tHEAD\n0123456789abcdef\tHEAD\n";

        assert_eq!(
            parse_advertised_default_branch(output).unwrap(),
            "release/next"
        );
    }

    #[test]
    fn usage_text_describes_the_run_command() {
        assert!(usage().contains("abacus run"));
        assert!(usage().contains("abacus land"));
        assert!(usage().contains("abacus drain"));
    }

    #[test]
    fn land_arguments_accept_once_on_either_side_of_the_optional_repo() {
        for args in [
            vec!["--once".to_owned(), "/tmp/repo".to_owned()],
            vec!["/tmp/repo".to_owned(), "--once".to_owned()],
        ] {
            assert_eq!(
                parse_land_args(&args).unwrap(),
                (PathBuf::from("/tmp/repo"), true)
            );
        }
        assert_eq!(parse_land_args(&[]).unwrap(), (PathBuf::from("."), false));
        assert!(parse_land_args(&["--unknown".into()]).is_err());
        assert!(parse_land_args(&["one".into(), "two".into()]).is_err());
    }

    #[test]
    fn resolution_prompt_carries_identity_attempt_provider_and_lane_owned_push_contract() {
        let candidate = Candidate {
            bead_id: "ab-resolution".into(),
            branch: "lane/ab-resolution".into(),
        };

        let prompt = resolution_prompt(&candidate, "trunk", "required check failed");

        for required in [
            "merge-queue resolution",
            "bead ab-resolution",
            "attempt 1 of 1",
            "existing PR branch lane/ab-resolution",
            "default branch trunk",
            "required check failed",
            "provider identity at every execution",
            "do not run any br write command",
            "push it from this lane",
            "abacus itself never pushes",
        ] {
            assert!(
                prompt.contains(required),
                "prompt lacked {required:?}: {prompt}"
            );
        }
    }

    #[test]
    fn outcome_probe_retry_waits_once_then_returns_the_second_result() {
        let mut probe_calls = 0;
        let mut delay_calls = 0;

        let outcome = retry_probe_once(
            || {
                probe_calls += 1;
                if probe_calls == 1 {
                    Err("first probe failed".to_owned())
                } else {
                    Ok(BeadOutcome::NeverEngaged)
                }
            },
            || delay_calls += 1,
        )
        .unwrap();

        assert_eq!(outcome, BeadOutcome::NeverEngaged);
        assert_eq!(probe_calls, 2, "only one re-probe is allowed");
        assert_eq!(delay_calls, 1, "the re-probe must be delayed once");
    }

    #[test]
    fn outcome_probe_retry_propagates_the_second_failure_unchanged() {
        let mut probe_calls = 0;
        let mut delay_calls = 0;

        let error = retry_probe_once::<BeadOutcome, _, _>(
            || {
                probe_calls += 1;
                Err(if probe_calls == 1 {
                    "first probe failed".to_owned()
                } else {
                    "second probe failed".to_owned()
                })
            },
            || delay_calls += 1,
        )
        .unwrap_err();

        assert_eq!(error, "second probe failed");
        assert_eq!(probe_calls, 2, "only one re-probe is allowed");
        assert_eq!(delay_calls, 1, "the re-probe must be delayed once");
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

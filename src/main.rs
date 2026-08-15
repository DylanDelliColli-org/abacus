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

use abacus::land::{
    Candidate, CompositionResult, DecisionInput, Eligibility, LandDecision, LocalLeg, QueueState,
    ValidationFailure, admission_red_park_body, decide, dequeue_park_body, enumerate_candidates,
    parse_eligibility, parse_enqueue_result, parse_queue_state,
};
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

fn retry_probe_once<T, Probe, Delay>(mut probe: Probe, delay: Delay) -> Result<T, String>
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

enum DispatchCycle {
    Empty,
    Completed,
    ClaimLost(String),
}

fn resolve_repo(repo: &Path) -> Result<PathBuf, String> {
    repo.canonicalize()
        .map_err(|e| format!("cannot resolve repo path {}: {e}", repo.display()))
}

fn cmd_run(repo: &Path) -> Result<(), String> {
    let repo = resolve_repo(repo)?;
    let repo_str = repo.to_string_lossy().into_owned();
    match dispatch_cycle(&repo, &repo_str, &BTreeSet::new(), false)? {
        DispatchCycle::Empty => {
            println!("no ready beads in {repo_str}; nothing to dispatch");
            Ok(())
        }
        DispatchCycle::Completed => Ok(()),
        DispatchCycle::ClaimLost(_) => unreachable!("run treats claim failures as errors"),
    }
}

fn cmd_drain(repo: &Path) -> Result<(), String> {
    let repo = resolve_repo(repo)?;
    let repo_str = repo.to_string_lossy().into_owned();
    let mut lost_claims = BTreeSet::new();

    loop {
        match dispatch_cycle(&repo, &repo_str, &lost_claims, true)? {
            DispatchCycle::Empty => {
                println!("no ready beads in {repo_str}; nothing to dispatch");
                return Ok(());
            }
            DispatchCycle::Completed => {}
            DispatchCycle::ClaimLost(bead_id) => {
                lost_claims.insert(bead_id);
            }
        }
    }
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
    let Some(bead) = select_bead(&claimable) else {
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
    let branch = format!("lane/{}", bead.id);
    let lane_started = Instant::now();
    let lane_result = (|| -> Result<(), String> {
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
                &agent_name,
                "--kind",
                "codex",
                "--pane",
                &lane.pane_id,
            ],
            None,
        )?;
        println!("codex worker started as agent {agent_name}");

        let prompt = dispatch_prompt(&bead.id, &lane.branch, &default_branch);
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

        let initial_outcome = probe_bead_outcome(repo, &bead.id)?;
        if initial_outcome == BeadOutcome::NeverEngaged {
            eprintln!("worker never engaged after startup prompt; retrying once");
        }
        let (retry_settled, outcome) = retry_never_engaged_once(
            initial_outcome,
            || capture("herdr", &prompt_args, None),
            || probe_bead_outcome(repo, &bead.id),
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
    })?;
    Ok(DispatchCycle::Completed)
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

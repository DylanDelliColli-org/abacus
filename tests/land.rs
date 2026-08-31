#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;
use std::{collections::BTreeSet, fs::OpenOptions, io::Write};

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "abacus-land-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn make_executable(path: &Path) {
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).unwrap();
}

fn read_log(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

fn relative_files(root: &Path) -> BTreeSet<PathBuf> {
    fn visit(root: &Path, directory: &Path, files: &mut BTreeSet<PathBuf>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path == root.join(".git") {
                continue;
            }
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(path.strip_prefix(root).unwrap().to_owned());
            }
        }
    }

    let mut files = BTreeSet::new();
    visit(root, root, &mut files);
    files
}

fn assert_no_forbidden_flags(gh_log: &Path, git_log: &Path) {
    let gh = read_log(gh_log);
    let git = read_log(git_log);

    for flag in ["--admin", "--match-head-commit", "-d", "--delete-branch"] {
        assert!(
            !gh.lines()
                .flat_map(|line| line.split_whitespace())
                .any(|arg| arg == flag),
            "forbidden gh flag {flag} in:\n{gh}"
        );
    }
    assert!(!gh.contains("pr update-branch"), "forbidden gh call:\n{gh}");
    for line in gh.lines().filter(|line| {
        line.starts_with("api ") && (line.contains("rulesets") || line.contains("protection"))
    }) {
        assert!(
            !line.split_whitespace().any(|arg| {
                matches!(arg, "--method" | "-X" | "POST" | "PUT" | "PATCH" | "DELETE")
            }),
            "forbidden mutating gh api call: {line}"
        );
    }

    for flag in [
        "push",
        "--force",
        "--force-with-lease",
        "-f",
        "-X",
        "--strategy-option",
    ] {
        assert!(
            !git.lines()
                .flat_map(|line| line.split_whitespace())
                .any(|arg| arg == flag),
            "forbidden git flag {flag} in:\n{git}"
        );
    }
    assert!(
        !git.lines()
            .any(|line| line.contains("-X ours") || line.contains("-X theirs")),
        "forbidden conflict-masking strategy in:\n{git}"
    );
}

fn run_land(repo: &Path, fake_bin: &Path) -> Output {
    let path = std::env::join_paths(std::iter::once(fake_bin.to_path_buf()).chain(
        std::env::split_paths(&std::env::var_os("PATH").expect("test PATH must be set")),
    ))
    .unwrap();
    run_land_with_retry(
        || {
            Command::new(env!("CARGO_BIN_EXE_abacus"))
                .args(["land", repo.to_str().unwrap(), "--once"])
                .env("PATH", &path)
                .env("ABACUS_LAND_POLL_MILLIS", "0")
                .output()
                .unwrap()
        },
        std::thread::sleep,
    )
}

fn run_land_with_retry<Run, Delay>(mut run: Run, delay: Delay) -> Output
where
    Run: FnMut() -> Output,
    Delay: FnOnce(Duration),
{
    const TEXT_FILE_BUSY: &str = "Text file busy";

    let first = run();
    if !first.status.success() && String::from_utf8_lossy(&first.stderr).contains(TEXT_FILE_BUSY) {
        delay(Duration::from_millis(100));
        let retry = run();
        assert!(
            retry.status.success(),
            "abacus land failed (retried once)\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&retry.stdout),
            String::from_utf8_lossy(&retry.stderr)
        );
        return retry;
    }

    first
}

#[test]
fn run_land_retries_text_file_busy_once_after_100_milliseconds() {
    let attempts = std::cell::Cell::new(0);
    let observed_delay = std::cell::Cell::new(None);

    let output = run_land_with_retry(
        || {
            attempts.set(attempts.get() + 1);
            if attempts.get() == 1 {
                shell_output(
                    "printf '%s' 'failed to spawn gh: Text file busy (os error 26)' >&2; exit 1",
                )
            } else {
                shell_output("printf '%s' retried-output")
            }
        },
        |delay| observed_delay.set(Some(delay)),
    );

    assert_eq!(attempts.get(), 2);
    assert_eq!(observed_delay.get(), Some(Duration::from_millis(100)));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "retried-output");
}

#[test]
fn run_land_does_not_retry_a_different_failure() {
    let attempts = std::cell::Cell::new(0);

    let output = run_land_with_retry(
        || {
            attempts.set(attempts.get() + 1);
            shell_output("printf '%s' 'failed to spawn gh: text file busy' >&2; exit 1")
        },
        |_| panic!("a non-matching failure must not be delayed or retried"),
    );

    assert_eq!(attempts.get(), 1);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("text file busy"));
}

#[test]
fn run_land_marks_a_failed_retry_loudly() {
    let attempts = std::cell::Cell::new(0);

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_land_with_retry(
            || {
                attempts.set(attempts.get() + 1);
                if attempts.get() == 1 {
                    shell_output("printf '%s' 'Text file busy' >&2; exit 1")
                } else {
                    shell_output("printf '%s' 'second land failure' >&2; exit 1")
                }
            },
            |_| {},
        );
    }))
    .expect_err("a failed retry must panic");

    assert_eq!(attempts.get(), 2);
    let message = panic_message(&panic);
    assert!(message.contains("retried once"));
    assert!(message.contains("second land failure"));
}

fn shell_output(script: &str) -> Output {
    Command::new("sh")
        .args(["-c", script])
        .output()
        .expect("sh must be available to exercise the land helper")
}

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> &str {
    panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .expect("panic must carry a string message")
}

fn run_ok(program: &str, args: &[&str], cwd: Option<&Path>) -> String {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{program} {} failed\nstdout: {}\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn git(repo: &Path, args: &[&str]) -> String {
    run_ok("git", args, Some(repo))
}

fn find_on_path(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("test PATH must be set"))
        .map(|directory| directory.join(program))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{program} must be on PATH"))
}

fn eligible_rulesets() -> &'static str {
    r#"[{"enforcement":"active","rules":[{"type":"merge_queue","parameters":{}},{"type":"required_status_checks","parameters":{"required_status_checks":[{"context":"test"}]}}]}]"#
}

fn queue_state(state: &str, queued: bool, merged: bool, reason: Option<&str>) -> String {
    let nodes: Vec<_> = reason
        .into_iter()
        .map(|reason| serde_json::json!({"reason": reason}))
        .collect();
    serde_json::json!({
        "data": {
            "repository": {
                "pullRequest": {
                    "state": state,
                    "merged": merged,
                    "isInMergeQueue": queued,
                    "autoMergeRequest": null,
                    "mergeQueueEntry": null,
                    "timelineItems": {"nodes": nodes}
                }
            }
        }
    })
    .to_string()
}

fn absent_queue_state() -> String {
    r#"{"data":{"repository":{"pullRequest":null}}}"#.into()
}

fn merged_queue_state() -> String {
    queue_state("MERGED", false, true, None)
}

fn queued_queue_state() -> String {
    queue_state("OPEN", true, false, None)
}

fn dequeued_queue_state(reason: &str) -> String {
    queue_state("OPEN", false, false, Some(reason))
}

struct LandFixture {
    _workspace: TempDir,
    repo: PathBuf,
    origin: PathBuf,
    fake_bin: PathBuf,
    gh_log: PathBuf,
    git_log: PathBuf,
    br_log: PathBuf,
    herdr_log: PathBuf,
    cargo_log: PathBuf,
    composition_log: PathBuf,
    open_prs: PathBuf,
    closed_beads: PathBuf,
    queue_dir: PathBuf,
    cargo_failure: PathBuf,
    herdr_action: PathBuf,
    default_branch: String,
}

impl LandFixture {
    fn new(tag: &str, default_branch: &str) -> Self {
        let workspace = TempDir::new(tag);
        let repo = workspace.0.join("repo");
        let origin = workspace.0.join("origin.git");
        let fake_bin = workspace.0.join("fake-bin");
        let queue_dir = workspace.0.join("queue");
        std::fs::create_dir_all(&fake_bin).unwrap();
        std::fs::create_dir_all(&queue_dir).unwrap();

        run_ok("git", &["init", "--bare", origin.to_str().unwrap()], None);
        run_ok(
            "git",
            &["init", "-b", default_branch, repo.to_str().unwrap()],
            None,
        );
        git(&repo, &["config", "user.name", "Abacus Test"]);
        git(&repo, &["config", "user.email", "abacus@example.test"]);
        std::fs::write(repo.join("base.txt"), "base\n").unwrap();
        git(&repo, &["add", "base.txt"]);
        git(&repo, &["commit", "-m", "base"]);
        git(
            &repo,
            &["remote", "add", "origin", origin.to_str().unwrap()],
        );
        git(&repo, &["push", "-u", "origin", default_branch]);
        run_ok(
            "git",
            &[
                "--git-dir",
                origin.to_str().unwrap(),
                "symbolic-ref",
                "HEAD",
                &format!("refs/heads/{default_branch}"),
            ],
            None,
        );

        let gh_log = workspace.0.join("gh.log");
        let git_log = workspace.0.join("git.log");
        let br_log = workspace.0.join("br.log");
        let herdr_log = workspace.0.join("herdr.log");
        let cargo_log = workspace.0.join("cargo.log");
        let composition_log = workspace.0.join("composition.log");
        let open_prs = workspace.0.join("open-prs.json");
        let closed_beads = workspace.0.join("closed-beads.json");
        let cargo_failure = workspace.0.join("cargo-failure");
        let herdr_action = workspace.0.join("herdr-action");
        std::fs::write(&open_prs, "[]\n").unwrap();
        std::fs::write(
            &closed_beads,
            r#"{"issues":[],"total":0,"limit":0,"offset":0,"has_more":false}"#,
        )
        .unwrap();
        std::fs::write(&cargo_failure, "none\n").unwrap();
        std::fs::write(&herdr_action, "none\n").unwrap();

        let gh = fake_bin.join("gh");
        std::fs::write(
            &gh,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{gh_log}'\n\
                 if [ \"$1 $2\" = \"repo view\" ]; then\n\
                   printf '%s\\n' '{{\"nameWithOwner\":\"owner/repo\",\"defaultBranchRef\":{{\"name\":\"{default_branch}\"}}}}'\n\
                 elif [ \"$1 $2\" = \"api repos/owner/repo/rulesets\" ]; then\n\
                   printf '%s\\n' '{rulesets}'\n\
                 elif [ \"$1 $2\" = \"pr list\" ]; then\n\
                   command cat '{open_prs}'\n\
                 elif [ \"$1 $2\" = \"pr view\" ]; then\n\
                   printf '%s\\n' '1'\n\
                 elif [ \"$1 $2\" = \"api graphql\" ]; then\n\
                   branch=''\n\
                   for arg in \"$@\"; do\n\
                     case \"$arg\" in branch=*) branch=${{arg#branch=}} ;; esac\n\
                   done\n\
                   bead=${{branch#lane/}}\n\
                   sequence='{queue_dir}/'$bead\n\
                   count_file=$sequence.count\n\
                   count=0\n\
                   if [ -f \"$count_file\" ]; then count=$(command cat \"$count_file\"); fi\n\
                   count=$((count + 1))\n\
                   printf '%s\\n' \"$count\" > \"$count_file\"\n\
                   line=$(command sed -n \"${{count}}p\" \"$sequence\" 2>/dev/null)\n\
                   if [ -z \"$line\" ]; then line=$(command tail -n 1 \"$sequence\" 2>/dev/null); fi\n\
                   if [ -z \"$line\" ]; then line='{absent}'; fi\n\
                   printf '%s\\n' \"$line\"\n\
                 elif [ \"$1 $2\" = \"pr merge\" ]; then\n\
                   printf '%s\\n' '✓ Pull request owner/repo#1 will be added to the merge queue for {default_branch} when ready'\n\
                 elif [ \"$1 $2\" = \"pr comment\" ]; then\n\
                   printf '%s\\n' 'commented'\n\
                 else\n\
                   printf 'unexpected gh call: %s\\n' \"$*\" >&2\n\
                   exit 2\n\
                 fi\n",
                gh_log = gh_log.display(),
                default_branch = default_branch,
                rulesets = eligible_rulesets(),
                open_prs = open_prs.display(),
                queue_dir = queue_dir.display(),
                absent = absent_queue_state(),
            ),
        )
        .unwrap();

        let br = fake_bin.join("br");
        std::fs::write(
            &br,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n\
                 if [ \"$1 $2\" = \"list --json\" ]; then\n\
                   command cat '{}'\n\
                 else\n\
                   printf 'unexpected br call: %s\\n' \"$*\" >&2\n\
                   exit 2\n\
                 fi\n",
                br_log.display(),
                closed_beads.display()
            ),
        )
        .unwrap();

        let real_git = find_on_path("git");
        let git_wrapper = fake_bin.join("git");
        std::fs::write(
            &git_wrapper,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexec '{}' \"$@\"\n",
                git_log.display(),
                real_git.display()
            ),
        )
        .unwrap();

        let cargo = fake_bin.join("cargo");
        std::fs::write(
            &cargo,
            format!(
                "#!/bin/sh\nprintf '%s|%s\\n' \"$PWD\" \"$*\" >> '{cargo_log}'\n\
                 head=$('{real_git}' rev-parse HEAD)\n\
                 base=$('{real_git}' rev-parse 'origin/{default_branch}')\n\
                 printf '%s|%s|%s\\n' \"$PWD\" \"$head\" \"$base\" >> '{composition_log}'\n\
                 failure=$(command cat '{cargo_failure}')\n\
                 if [ \"$failure\" = \"$1\" ]; then\n\
                   printf 'injected %s failure on composition %s\\n' \"$1\" \"$head\" >&2\n\
                   exit 1\n\
                 fi\n",
                cargo_log = cargo_log.display(),
                real_git = real_git.display(),
                default_branch = default_branch,
                composition_log = composition_log.display(),
                cargo_failure = cargo_failure.display(),
            ),
        )
        .unwrap();

        let herdr = fake_bin.join("herdr");
        std::fs::write(
            &herdr,
            format!(
                "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> '{herdr_log}'\n\
                 resolve_branch() {{\n\
                   branch=$1\n\
                   printf 'resolution-action=%s branch=%s\\n' \"$action\" \"$branch\" >> '{herdr_log}'\n\
                   parent=$('{real_git}' --git-dir='{origin}' rev-parse \"refs/heads/$branch\")\n\
                   tree=$('{real_git}' --git-dir='{origin}' rev-parse \"$parent^{{tree}}\")\n\
                   commit=$(GIT_AUTHOR_NAME='Resolution Agent' GIT_AUTHOR_EMAIL='agent@example.test' GIT_COMMITTER_NAME='Resolution Agent' GIT_COMMITTER_EMAIL='agent@example.test' '{real_git}' --git-dir='{origin}' commit-tree \"$tree\" -p \"$parent\" -m 'agent resolution')\n\
                   '{real_git}' --git-dir='{origin}' update-ref \"refs/heads/$branch\" \"$commit\"\n\
                 }}\n\
                 if [ \"$1 $2\" = \"worktree open\" ]; then\n\
                   branch=''\n\
                   for arg in \"$@\"; do\n\
                     case \"$arg\" in lane/*) branch=$arg ;; esac\n\
                   done\n\
                   printf '%s\\n' '{{\"result\":{{\"type\":\"worktree_opened\",\"workspace\":{{\"workspace_id\":\"resolution-workspace\"}},\"root_pane\":{{\"pane_id\":\"resolution-pane\"}},\"worktree\":{{\"path\":\"{repo}\",\"branch\":\"'$branch'\"}}}}}}'\n\
                 elif [ \"$1 $2\" = \"agent start\" ]; then\n\
                   exit 0\n\
                 elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
                   action=$(command cat '{herdr_action}')\n\
                   if [ \"$action\" = \"fail\" ]; then\n\
                     printf 'injected resolution-agent failure\\n' >&2\n\
                     exit 1\n\
                   fi\n\
                   if [ \"$action\" = \"resolve\" ] || [ \"$action\" = \"pasted-resolve\" ]; then\n\
                     branch=$(command sed -n 's|.*existing PR branch \\(lane/[A-Za-z0-9._-]*\\).*|\\1|p' <<EOF\n\
$*\n\
EOF\n\
                     )\n\
                     branch=${{branch%.}}\n\
                     printf '%s\\n' \"$branch\" > '{herdr_action}.branch'\n\
                     if [ \"$action\" = \"resolve\" ]; then resolve_branch \"$branch\"; fi\n\
                   fi\n\
                   printf '%s\\n' 'resolution agent settled'\n\
                 elif [ \"$1 $2\" = \"pane read\" ]; then\n\
                   action=$(command cat '{herdr_action}')\n\
                   if [ \"$action\" = \"pasted-resolve\" ]; then\n\
                     printf '› [Pasted Content 888 chars]\\n\\n  gpt-5.6-sol high · Context 0%% used\\n'\n\
                   else\n\
                     printf '› Ask Codex to do anything\\n\\n  gpt-5.6-sol high · Context 24%% used\\n'\n\
                   fi\n\
                 elif [ \"$1 $2\" = \"agent send-keys\" ]; then\n\
                   action=$(command cat '{herdr_action}')\n\
                   if [ \"$action\" != \"pasted-resolve\" ]; then printf 'unexpected Enter nudge\\n' >&2; exit 2; fi\n\
                   branch=$(command cat '{herdr_action}.branch')\n\
                   resolve_branch \"$branch\"\n\
                 elif [ \"$1 $2\" = \"agent wait\" ]; then\n\
                   printf 'resolution agent transition observed\\n'\n\
                 else\n\
                   printf 'unexpected herdr call: %s\\n' \"$*\" >&2\n\
                   exit 2\n\
                 fi\n",
                herdr_log = herdr_log.display(),
                repo = repo.display(),
                herdr_action = herdr_action.display(),
                real_git = real_git.display(),
                origin = origin.display(),
            ),
        )
        .unwrap();

        for program in [&gh, &br, &git_wrapper, &cargo, &herdr] {
            make_executable(program);
        }

        Self {
            _workspace: workspace,
            repo,
            origin,
            fake_bin,
            gh_log,
            git_log,
            br_log,
            herdr_log,
            cargo_log,
            composition_log,
            open_prs,
            closed_beads,
            queue_dir,
            cargo_failure,
            herdr_action,
            default_branch: default_branch.into(),
        }
    }

    fn add_lane(&self, bead_id: &str) -> String {
        let branch = format!("lane/{bead_id}");
        git(&self.repo, &["checkout", "-b", &branch]);
        let feature = self.repo.join(format!("feature-{bead_id}.txt"));
        std::fs::write(&feature, format!("feature for {bead_id}\n")).unwrap();
        git(
            &self.repo,
            &["add", feature.file_name().unwrap().to_str().unwrap()],
        );
        git(&self.repo, &["commit", "-m", &format!("feature {bead_id}")]);
        git(&self.repo, &["push", "-u", "origin", &branch]);
        let head = git(&self.repo, &["rev-parse", "HEAD"]);
        git(&self.repo, &["checkout", &self.default_branch]);
        head
    }

    fn advance_default(&self, marker: &str) -> String {
        let filename = format!("default-{marker}.txt");
        std::fs::write(self.repo.join(&filename), format!("{marker}\n")).unwrap();
        git(&self.repo, &["add", &filename]);
        git(
            &self.repo,
            &["commit", "-m", &format!("advance default {marker}")],
        );
        git(&self.repo, &["push", "origin", &self.default_branch]);
        git(&self.repo, &["rev-parse", "HEAD"])
    }

    fn add_conflicting_lane(&self, bead_id: &str) -> String {
        let conflict_file = self.repo.join("conflict.txt");
        std::fs::write(&conflict_file, "shared base\n").unwrap();
        git(&self.repo, &["add", "conflict.txt"]);
        git(&self.repo, &["commit", "-m", "add conflict base"]);
        git(&self.repo, &["push", "origin", &self.default_branch]);

        let branch = format!("lane/{bead_id}");
        git(&self.repo, &["checkout", "-b", &branch]);
        std::fs::write(&conflict_file, "lane version\n").unwrap();
        git(&self.repo, &["add", "conflict.txt"]);
        git(&self.repo, &["commit", "-m", "lane conflict hunk"]);
        git(&self.repo, &["push", "-u", "origin", &branch]);
        let head = git(&self.repo, &["rev-parse", "HEAD"]);

        git(&self.repo, &["checkout", &self.default_branch]);
        std::fs::write(&conflict_file, "default version\n").unwrap();
        git(&self.repo, &["add", "conflict.txt"]);
        git(&self.repo, &["commit", "-m", "default conflict hunk"]);
        git(&self.repo, &["push", "origin", &self.default_branch]);
        head
    }

    fn add_jsonl_conflict_lane(&self, bead_id: &str) -> (String, PathBuf) {
        let beads = self.repo.join(".beads");
        std::fs::create_dir_all(&beads).unwrap();
        std::fs::write(
            self.repo.join(".gitattributes"),
            ".beads/issues.jsonl merge=beads-jsonl\n",
        )
        .unwrap();
        std::fs::write(
            beads.join("issues.jsonl"),
            "{\"id\":\"ab-base\",\"updated_at\":\"2026-08-15T00:00:00Z\"}\n",
        )
        .unwrap();
        let driver_log = self._workspace.0.join("merge-driver.log");
        let driver = self._workspace.0.join("merge-jsonl-driver");
        std::fs::write(
            &driver,
            format!(
                "#!/bin/sh\nset -eu\nprintf 'called\\n' >> '{}'\ncommand sort -u \"$1\" \"$3\" -o \"$1\"\n",
                driver_log.display()
            ),
        )
        .unwrap();
        make_executable(&driver);
        git(
            &self.repo,
            &[
                "config",
                "merge.beads-jsonl.driver",
                &format!("'{}' %A %O %B", driver.display()),
            ],
        );
        git(
            &self.repo,
            &["add", ".gitattributes", ".beads/issues.jsonl"],
        );
        git(
            &self.repo,
            &["commit", "-m", "configure tracker merge driver"],
        );
        git(&self.repo, &["push", "origin", &self.default_branch]);

        let branch = format!("lane/{bead_id}");
        git(&self.repo, &["checkout", "-b", &branch]);
        writeln!(
            OpenOptions::new()
                .append(true)
                .open(beads.join("issues.jsonl"))
                .unwrap(),
            "{{\"id\":\"{bead_id}\",\"updated_at\":\"2026-08-15T01:00:00Z\"}}"
        )
        .unwrap();
        git(&self.repo, &["add", ".beads/issues.jsonl"]);
        git(&self.repo, &["commit", "-m", "append lane bead"]);
        git(&self.repo, &["push", "-u", "origin", &branch]);
        let head = git(&self.repo, &["rev-parse", "HEAD"]);

        git(&self.repo, &["checkout", &self.default_branch]);
        writeln!(
            OpenOptions::new()
                .append(true)
                .open(beads.join("issues.jsonl"))
                .unwrap(),
            "{{\"id\":\"ab-default\",\"updated_at\":\"2026-08-15T02:00:00Z\"}}"
        )
        .unwrap();
        git(&self.repo, &["add", ".beads/issues.jsonl"]);
        git(&self.repo, &["commit", "-m", "append default bead"]);
        git(&self.repo, &["push", "origin", &self.default_branch]);
        (head, driver_log)
    }

    fn candidates(&self, pr_beads: &[&str], closed_beads: &[&str]) {
        let prs: Vec<_> = pr_beads
            .iter()
            .map(|bead| serde_json::json!({"headRefName": format!("lane/{bead}")}))
            .collect();
        let beads: Vec<_> = closed_beads
            .iter()
            .map(|bead| serde_json::json!({"id": bead, "status": "closed"}))
            .collect();
        std::fs::write(&self.open_prs, serde_json::to_vec(&prs).unwrap()).unwrap();
        std::fs::write(
            &self.closed_beads,
            serde_json::to_vec(&serde_json::json!({
                "issues": beads,
                "total": beads.len(),
                "limit": 0,
                "offset": 0,
                "has_more": false
            }))
            .unwrap(),
        )
        .unwrap();
    }

    fn queue(&self, bead_id: &str, states: &[String]) {
        let mut contents = states.join("\n");
        contents.push('\n');
        std::fs::write(self.queue_dir.join(bead_id), contents).unwrap();
    }

    fn run(&self) -> Output {
        run_land(&self.repo, &self.fake_bin)
    }

    fn origin_head(&self, branch: &str) -> String {
        run_ok(
            "git",
            &[
                "--git-dir",
                self.origin.to_str().unwrap(),
                "rev-parse",
                &format!("refs/heads/{branch}"),
            ],
            None,
        )
    }

    fn assert_forbidden_invariant(&self) {
        assert_no_forbidden_flags(&self.gh_log, &self.git_log);
    }
}

#[test]
fn t3_ineligible_repository_refuses_before_worktree_or_enqueue_side_effects() {
    let workspace = TempDir::new("ineligible");
    let repo = workspace.0.join("repo");
    let fake_bin = workspace.0.join("fake-bin");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::create_dir_all(&fake_bin).unwrap();

    let gh_log = workspace.0.join("gh.log");
    let git_log = workspace.0.join("git.log");
    let br_log = workspace.0.join("br.log");
    let herdr_log = workspace.0.join("herdr.log");

    let gh = fake_bin.join("gh");
    std::fs::write(
        &gh,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n\
             case \"$1 $2\" in\n\
               'repo view') printf '%s\\n' '{{\"nameWithOwner\":\"owner/repo\",\"defaultBranchRef\":{{\"name\":\"main\"}}}}' ;;\n\
               'api repos/owner/repo/rulesets') printf '%s\\n' '[]' ;;\n\
               *) printf 'unexpected gh call: %s\\n' \"$*\" >&2; exit 2 ;;\n\
             esac\n",
            gh_log.display()
        ),
    )
    .unwrap();

    for (name, log) in [("git", &git_log), ("br", &br_log), ("herdr", &herdr_log)] {
        let path = fake_bin.join(name);
        std::fs::write(
            &path,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nprintf 'unexpected {name} call: %s\\n' \"$*\" >&2\nexit 2\n",
                log.display()
            ),
        )
        .unwrap();
        make_executable(&path);
    }
    make_executable(&gh);

    let output = run_land(&repo, &fake_bin);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("merge queue"), "stderr: {stderr}");
    assert!(
        !read_log(&git_log).contains("worktree add"),
        "eligibility refusal happened after worktree creation"
    );
    assert!(
        !read_log(&gh_log).contains("pr merge"),
        "eligibility refusal happened after enqueue"
    );
    assert!(
        read_log(&br_log).is_empty(),
        "br was read before eligibility"
    );
    assert!(
        read_log(&herdr_log).is_empty(),
        "Herdr was touched before eligibility"
    );
    assert_no_forbidden_flags(&gh_log, &git_log);
}

#[test]
fn t4_t41_t47_t52_happy_admission_uses_fresh_unpushed_composition_and_bare_enqueue() {
    for default_branch in ["main", "trunk"] {
        let fixture = LandFixture::new(&format!("happy-{default_branch}"), default_branch);
        let branch_head = fixture.add_lane("ab-happy");
        let default_tip = fixture.advance_default("one");
        fixture.candidates(&["ab-happy"], &["ab-happy"]);
        fixture.queue("ab-happy", &[absent_queue_state(), merged_queue_state()]);

        let output = fixture.run();
        assert!(
            output.status.success(),
            "default={default_branch}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let cargo_calls: Vec<_> = read_log(&fixture.cargo_log)
            .lines()
            .map(str::to_owned)
            .collect();
        assert_eq!(cargo_calls.len(), 3, "cargo calls: {cargo_calls:#?}");
        assert!(cargo_calls[0].ends_with("|test"), "{cargo_calls:#?}");
        assert!(
            cargo_calls[1].ends_with("|clippy --all-targets --all-features -- -D warnings"),
            "{cargo_calls:#?}"
        );
        assert!(cargo_calls[2].ends_with("|fmt --check"), "{cargo_calls:#?}");
        let admission_path = cargo_calls[0].split('|').next().unwrap();
        assert_ne!(admission_path, fixture.repo.to_str().unwrap());
        assert!(
            cargo_calls
                .iter()
                .all(|call| call.starts_with(&format!("{admission_path}|"))),
            "validation legs used different worktrees: {cargo_calls:#?}"
        );

        let first_composition = read_log(&fixture.composition_log)
            .lines()
            .next()
            .unwrap()
            .split('|')
            .nth(1)
            .unwrap()
            .to_owned();
        git(
            &fixture.repo,
            &[
                "merge-base",
                "--is-ancestor",
                &default_tip,
                &first_composition,
            ],
        );
        assert_eq!(
            fixture.origin_head("lane/ab-happy"),
            branch_head,
            "admission changed the remote PR head"
        );

        let gh_calls = read_log(&fixture.gh_log);
        let enqueue_calls: Vec<_> = gh_calls
            .lines()
            .filter(|line| line.starts_with("pr merge "))
            .collect();
        assert_eq!(enqueue_calls, ["pr merge lane/ab-happy"]);
        let git_calls = read_log(&fixture.git_log);
        assert!(
            git_calls.contains("fetch origin"),
            "git calls:\n{git_calls}"
        );
        assert!(
            git_calls
                .lines()
                .any(|line| line.starts_with("worktree add --detach ")),
            "git calls:\n{git_calls}"
        );
        assert!(
            git_calls.contains(&format!("merge origin/{default_branch}")),
            "default branch was not composed dynamically:\n{git_calls}"
        );
        assert!(
            git_calls
                .lines()
                .any(|line| line.starts_with("worktree remove ")),
            "admission worktree was not removed:\n{git_calls}"
        );
        let worktrees = git(&fixture.repo, &["worktree", "list", "--porcelain"]);
        assert_eq!(
            worktrees
                .lines()
                .filter(|line| line.starts_with("worktree "))
                .count(),
            1,
            "admission worktree survived:\n{worktrees}"
        );
        assert!(read_log(&fixture.herdr_log).is_empty());
        fixture.assert_forbidden_invariant();
    }
}

#[test]
fn t5_red_clippy_admission_parks_with_evidence_without_enqueue_or_push() {
    let fixture = LandFixture::new("red-clippy", "main");
    let branch_head = fixture.add_lane("ab-red");
    fixture.advance_default("red-base");
    fixture.candidates(&["ab-red"], &["ab-red"]);
    fixture.queue("ab-red", &[absent_queue_state()]);
    std::fs::write(&fixture.cargo_failure, "clippy\n").unwrap();

    let output = fixture.run();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let cargo = read_log(&fixture.cargo_log);
    assert!(cargo.lines().any(|line| line.ends_with("|test")), "{cargo}");
    assert!(
        cargo
            .lines()
            .any(|line| line.ends_with("|clippy --all-targets --all-features -- -D warnings")),
        "{cargo}"
    );
    assert!(
        !cargo.lines().any(|line| line.ends_with("|fmt --check")),
        "{cargo}"
    );

    let gh = read_log(&fixture.gh_log);
    assert!(
        !gh.lines().any(|line| line.starts_with("pr merge ")),
        "{gh}"
    );
    assert!(
        gh.contains(&format!(
            "pr comment lane/ab-red --body Parking bead ab-red at admitted head {branch_head}"
        )),
        "park comment lacked bead/head evidence:\n{gh}"
    );
    assert!(
        gh.contains("cargo clippy"),
        "park comment lacked tool evidence:\n{gh}"
    );
    assert!(
        gh.contains("injected clippy failure"),
        "park comment lacked stderr evidence:\n{gh}"
    );
    assert_eq!(fixture.origin_head("lane/ab-red"), branch_head);
    assert!(read_log(&fixture.herdr_log).is_empty());
    let worktrees = git(&fixture.repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(
        worktrees
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count(),
        1,
        "admission worktree survived:\n{worktrees}"
    );
    fixture.assert_forbidden_invariant();
}

#[test]
fn t7_each_once_cycle_refetches_and_composes_the_new_default_tip() {
    let fixture = LandFixture::new("fresh-composition", "main");
    let branch_head = fixture.add_lane("ab-fresh");
    let first_tip = fixture.advance_default("first");
    fixture.candidates(&["ab-fresh"], &["ab-fresh"]);
    fixture.queue(
        "ab-fresh",
        &[
            absent_queue_state(),
            merged_queue_state(),
            absent_queue_state(),
            merged_queue_state(),
        ],
    );

    let first = fixture.run();
    assert!(
        first.status.success(),
        "first stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let second_tip = fixture.advance_default("second");
    let second = fixture.run();
    assert!(
        second.status.success(),
        "second stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    let compositions: Vec<_> = read_log(&fixture.composition_log)
        .lines()
        .map(|line| line.split('|').nth(1).unwrap().to_owned())
        .collect();
    assert_eq!(compositions.len(), 6, "composition log: {compositions:#?}");
    git(
        &fixture.repo,
        &["merge-base", "--is-ancestor", &first_tip, &compositions[0]],
    );
    git(
        &fixture.repo,
        &["merge-base", "--is-ancestor", &second_tip, &compositions[3]],
    );
    let stale_check = Command::new("git")
        .args(["merge-base", "--is-ancestor", &second_tip, &compositions[0]])
        .current_dir(&fixture.repo)
        .output()
        .unwrap();
    assert!(
        !stale_check.status.success(),
        "first composition somehow contained a future default tip"
    );
    assert_eq!(
        read_log(&fixture.git_log)
            .lines()
            .filter(|line| *line == "fetch origin")
            .count(),
        2
    );
    assert_eq!(fixture.origin_head("lane/ab-fresh"), branch_head);
    fixture.assert_forbidden_invariant();
}

#[test]
fn t12_closed_bead_without_a_pr_is_a_normal_skip() {
    let fixture = LandFixture::new("prless-skip", "main");
    fixture.add_lane("ab-with-pr");
    fixture.candidates(&["ab-with-pr"], &["ab-with-pr", "ab-without-pr"]);
    fixture.queue("ab-with-pr", &[absent_queue_state(), merged_queue_state()]);

    let output = fixture.run();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let gh = read_log(&fixture.gh_log);
    let enqueue_calls: Vec<_> = gh
        .lines()
        .filter(|line| line.starts_with("pr merge "))
        .collect();
    assert_eq!(enqueue_calls, ["pr merge lane/ab-with-pr"]);
    assert!(
        !gh.contains("ab-without-pr"),
        "PR-less bead leaked into gh calls:\n{gh}"
    );
    assert_eq!(read_log(&fixture.br_log).trim(), "list --json");
    assert!(read_log(&fixture.herdr_log).is_empty());
    fixture.assert_forbidden_invariant();
}

#[test]
fn t15_initially_dequeued_pr_gets_one_failed_resolution_then_parks_without_enqueue() {
    let fixture = LandFixture::new("initially-dequeued", "main");
    let branch_head = fixture.add_lane("ab-dequeued");
    fixture.candidates(&["ab-dequeued"], &["ab-dequeued"]);
    fixture.queue(
        "ab-dequeued",
        &[dequeued_queue_state("required check test failed")],
    );
    std::fs::write(&fixture.herdr_action, "fail\n").unwrap();

    let output = fixture.run();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let gh = read_log(&fixture.gh_log);
    assert!(
        !gh.lines().any(|line| line.starts_with("pr merge ")),
        "{gh}"
    );
    assert!(gh.contains("Parking bead ab-dequeued"), "{gh}");
    assert!(
        gh.contains(&branch_head),
        "park comment lacked admitted SHA:\n{gh}"
    );
    assert!(gh.contains("required check test failed"), "{gh}");

    let herdr = read_log(&fixture.herdr_log);
    assert_eq!(
        herdr
            .lines()
            .filter(|line| line.starts_with("worktree open "))
            .count(),
        1,
        "{herdr}"
    );
    assert_eq!(
        herdr
            .lines()
            .filter(|line| line.starts_with("agent start "))
            .count(),
        1,
        "{herdr}"
    );
    assert_eq!(
        herdr
            .lines()
            .filter(|line| line.starts_with("agent prompt "))
            .count(),
        1,
        "{herdr}"
    );
    assert!(
        herdr.contains("ab-dequeued"),
        "prompt lacked bead identity:\n{herdr}"
    );
    assert!(
        herdr.contains("attempt 1 of 1"),
        "prompt lacked attempt marker:\n{herdr}"
    );
    assert!(
        herdr.contains("merge-queue resolution"),
        "prompt lacked resolution framing:\n{herdr}"
    );
    assert_eq!(read_log(&fixture.br_log).trim(), "list --json");
    assert_eq!(fixture.origin_head("lane/ab-dequeued"), branch_head);
    fixture.assert_forbidden_invariant();
}

#[test]
fn t50_queued_then_dequeued_dispatches_once_and_does_not_reenqueue_before_failure() {
    let fixture = LandFixture::new("queued-dequeued", "main");
    fixture.add_lane("ab-watch");
    fixture.candidates(&["ab-watch"], &["ab-watch"]);
    fixture.queue(
        "ab-watch",
        &[
            absent_queue_state(),
            queued_queue_state(),
            dequeued_queue_state("merge-group test failed"),
        ],
    );
    std::fs::write(&fixture.herdr_action, "fail\n").unwrap();

    let output = fixture.run();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let gh = read_log(&fixture.gh_log);
    assert_eq!(
        gh.lines()
            .filter(|line| line.starts_with("pr merge "))
            .count(),
        1,
        "a second enqueue occurred before the one attempt terminated:\n{gh}"
    );
    assert_eq!(
        gh.lines().filter(|line| *line == "api graphql -f query=query($owner:String!,$name:String!,$number:Int!,$branch:String!){repository(owner:$owner,name:$name){ref(qualifiedName:$branch){name} pullRequest(number:$number){state merged isInMergeQueue autoMergeRequest{enabledAt} mergeQueueEntry{id} timelineItems(last:1,itemTypes:[REMOVED_FROM_MERGE_QUEUE_EVENT]){nodes{... on RemovedFromMergeQueueEvent{reason}}}}}} -F owner=owner -F name=repo -F number=1 -F branch=lane/ab-watch").count(),
        3,
        "expected pre-admission plus two injected-delay watch reads:\n{gh}"
    );
    let herdr = read_log(&fixture.herdr_log);
    assert_eq!(
        herdr
            .lines()
            .filter(|line| line.starts_with("worktree open "))
            .count(),
        1,
        "{herdr}"
    );
    assert_eq!(
        herdr
            .lines()
            .filter(|line| line.starts_with("agent prompt "))
            .count(),
        1,
        "{herdr}"
    );
    fixture.assert_forbidden_invariant();
}

#[test]
fn t51_green_resolution_attempt_is_readmitted_and_reenqueued_exactly_once() {
    let fixture = LandFixture::new("green-resolution", "main");
    let initial_head = fixture.add_lane("ab-resolved");
    fixture.advance_default("resolution-base");
    fixture.candidates(&["ab-resolved"], &["ab-resolved"]);
    fixture.queue(
        "ab-resolved",
        &[
            absent_queue_state(),
            queued_queue_state(),
            dequeued_queue_state("merge-group clippy failed"),
            merged_queue_state(),
        ],
    );
    std::fs::write(&fixture.herdr_action, "pasted-resolve\n").unwrap();

    let output = fixture.run();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let resolved_head = fixture.origin_head("lane/ab-resolved");
    assert_ne!(
        resolved_head,
        initial_head,
        "fake resolution lane did not push:\n{}",
        read_log(&fixture.herdr_log)
    );
    let gh = read_log(&fixture.gh_log);
    assert_eq!(
        gh.lines()
            .filter(|line| line.starts_with("pr merge "))
            .count(),
        2,
        "expected initial enqueue plus one post-resolution enqueue:\n{gh}"
    );
    assert!(
        !gh.lines().any(|line| line.starts_with("pr comment ")),
        "{gh}"
    );
    let herdr = read_log(&fixture.herdr_log);
    assert_eq!(
        herdr
            .lines()
            .filter(|line| line.starts_with("worktree open "))
            .count(),
        1,
        "{herdr}"
    );
    for recovery_call in [
        "pane read resolution-pane --lines 40",
        "agent send-keys r-ab-resolved Enter",
        "agent wait r-ab-resolved --until working --timeout 5000",
        "agent wait r-ab-resolved --until done --until blocked",
    ] {
        assert!(
            herdr.lines().any(|line| line == recovery_call),
            "land resolution bypassed shared prompt recovery call {recovery_call:?}:\n{herdr}"
        );
    }
    assert_eq!(
        herdr
            .lines()
            .filter(|line| line.starts_with("agent prompt "))
            .count(),
        1,
        "{herdr}"
    );
    assert_eq!(
        read_log(&fixture.cargo_log).lines().count(),
        6,
        "initial admission and readmission must both run all local legs"
    );
    fixture.assert_forbidden_invariant();
}

#[test]
fn t23_conflicting_source_hunks_dispatch_once_then_park_when_agent_leaves_unresolved() {
    let fixture = LandFixture::new("source-conflict", "main");
    let branch_head = fixture.add_conflicting_lane("ab-conflict");
    fixture.candidates(&["ab-conflict"], &["ab-conflict"]);
    fixture.queue("ab-conflict", &[absent_queue_state()]);

    let output = fixture.run();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let gh = read_log(&fixture.gh_log);
    assert!(
        !gh.lines().any(|line| line.starts_with("pr merge ")),
        "{gh}"
    );
    assert!(gh.contains("Parking bead ab-conflict"), "{gh}");
    assert!(
        gh.contains(&branch_head),
        "park comment lacked admitted SHA:\n{gh}"
    );
    assert!(
        gh.contains("composition conflict"),
        "park comment lacked conflict reason:\n{gh}"
    );
    let herdr = read_log(&fixture.herdr_log);
    assert_eq!(
        herdr
            .lines()
            .filter(|line| line.starts_with("worktree open "))
            .count(),
        1,
        "{herdr}"
    );
    assert_eq!(
        herdr
            .lines()
            .filter(|line| line.starts_with("agent prompt "))
            .count(),
        1,
        "{herdr}"
    );
    assert!(read_log(&fixture.cargo_log).is_empty());
    assert_eq!(fixture.origin_head("lane/ab-conflict"), branch_head);
    let worktrees = git(&fixture.repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(
        worktrees
            .lines()
            .filter(|line| line.starts_with("worktree "))
            .count(),
        1,
        "conflicted admission worktree survived:\n{worktrees}"
    );
    fixture.assert_forbidden_invariant();
}

#[test]
fn t22_tracker_jsonl_conflict_uses_inherited_merge_driver_without_agent_resolution() {
    let fixture = LandFixture::new("jsonl-driver", "main");
    let (branch_head, driver_log) = fixture.add_jsonl_conflict_lane("ab-jsonl");
    fixture.candidates(&["ab-jsonl"], &["ab-jsonl"]);
    fixture.queue("ab-jsonl", &[absent_queue_state(), merged_queue_state()]);

    let output = fixture.run();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(read_log(&driver_log).trim(), "called");
    let gh = read_log(&fixture.gh_log);
    assert_eq!(
        gh.lines()
            .filter(|line| line.starts_with("pr merge "))
            .collect::<Vec<_>>(),
        ["pr merge lane/ab-jsonl"]
    );
    assert!(read_log(&fixture.herdr_log).is_empty());
    assert_eq!(read_log(&fixture.cargo_log).lines().count(), 3);
    assert_eq!(fixture.origin_head("lane/ab-jsonl"), branch_head);
    fixture.assert_forbidden_invariant();
}

#[test]
fn t44_queued_and_merged_candidates_are_not_reenqueued_and_no_state_file_is_written() {
    let fixture = LandFixture::new("stateless-skips", "main");
    fixture.add_lane("ab-queued");
    fixture.add_lane("ab-merged");
    fixture.add_lane("ab-new");
    fixture.candidates(
        &["ab-queued", "ab-merged", "ab-new"],
        &["ab-queued", "ab-merged", "ab-new"],
    );
    fixture.queue("ab-queued", &[queued_queue_state()]);
    fixture.queue("ab-merged", &[merged_queue_state()]);
    fixture.queue("ab-new", &[absent_queue_state(), merged_queue_state()]);
    let files_before = relative_files(&fixture.repo);

    let output = fixture.run();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let gh = read_log(&fixture.gh_log);
    assert_eq!(
        gh.lines()
            .filter(|line| line.starts_with("pr merge "))
            .collect::<Vec<_>>(),
        ["pr merge lane/ab-new"]
    );
    assert!(read_log(&fixture.herdr_log).is_empty());
    assert_eq!(read_log(&fixture.cargo_log).lines().count(), 3);
    assert_eq!(
        relative_files(&fixture.repo),
        files_before,
        "land persisted state in the repository"
    );
    assert_eq!(read_log(&fixture.br_log).trim(), "list --json");
    fixture.assert_forbidden_invariant();
}

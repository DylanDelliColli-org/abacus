//! Integration: real `br` binary against a throwaway workspace, and the real
//! `abacus` binary against real backlogs. Tests that need the pinned `br`
//! substrate return early when it is not resolvable on PATH, while portable
//! contract tests continue to run.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};

use abacus::{BeadOutcome, dispatch_prompt, parse_bead_outcome, parse_ready, select_bead};

struct TempWorkspace(PathBuf);

impl TempWorkspace {
    fn new(tag: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = (0..1_024)
            .find_map(|attempt| {
                let candidate = std::env::temp_dir().join(format!(
                    "abacus-it-{tag}-{}-{nonce}-{attempt}",
                    std::process::id()
                ));
                match std::fs::create_dir(&candidate) {
                    Ok(()) => Some(candidate),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => None,
                    Err(error) => panic!(
                        "failed to create temporary workspace {}: {error}",
                        candidate.display()
                    ),
                }
            })
            .unwrap_or_else(|| panic!("failed to allocate a unique temporary workspace for {tag}"));
        git(&dir, &["init", "--quiet", "--initial-branch", "main"]);
        git(
            &dir,
            &[
                "symbolic-ref",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/main",
            ],
        );
        TempWorkspace(dir)
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn br(dir: &PathBuf, args: &[&str]) -> String {
    let out = br_with_retry(
        args,
        || {
            Command::new("br")
                .args(args)
                .current_dir(dir)
                .output()
                .expect("br must be on PATH")
        },
        std::thread::sleep,
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn br_with_retry<Run, Delay>(args: &[&str], mut run: Run, delay: Delay) -> Output
where
    Run: FnMut() -> Output,
    Delay: FnOnce(Duration),
{
    const TIMESTAMP_RACE: &str = "updated_at: cannot be before created_at";

    let first = run();
    if first.status.success() {
        return first;
    }

    if String::from_utf8_lossy(&first.stderr).contains(TIMESTAMP_RACE) {
        delay(Duration::from_millis(100));
        let retry = run();
        assert!(
            retry.status.success(),
            "br {args:?} failed (retried once): {}",
            String::from_utf8_lossy(&retry.stderr)
        );
        return retry;
    }

    assert!(
        first.status.success(),
        "br {args:?} failed: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    first
}

#[test]
fn br_helper_retries_the_timestamp_race_once_after_100_milliseconds() {
    let attempts = std::cell::Cell::new(0);
    let observed_delay = std::cell::Cell::new(None);

    let output = br_with_retry(
        &["create", "--title=test"],
        || {
            attempts.set(attempts.get() + 1);
            if attempts.get() == 1 {
                shell_output("printf '%s' 'updated_at: cannot be before created_at' >&2; exit 1")
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
fn br_helper_does_not_retry_a_different_failure() {
    let attempts = std::cell::Cell::new(0);

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        br_with_retry(
            &["create", "--title=test"],
            || {
                attempts.set(attempts.get() + 1);
                shell_output("printf '%s' 'updated_at cannot be before created_at' >&2; exit 1")
            },
            |_| panic!("a non-matching failure must not be delayed or retried"),
        );
    }))
    .expect_err("the original br failure must panic");

    assert_eq!(attempts.get(), 1);
    let message = panic_message(&panic);
    assert!(message.contains("br [\"create\", \"--title=test\"] failed:"));
    assert!(!message.contains("retried once"));
}

#[test]
fn br_helper_marks_a_failed_retry_loudly() {
    let attempts = std::cell::Cell::new(0);

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        br_with_retry(
            &["create", "--title=test"],
            || {
                attempts.set(attempts.get() + 1);
                if attempts.get() == 1 {
                    shell_output(
                        "printf '%s' 'updated_at: cannot be before created_at' >&2; exit 1",
                    )
                } else {
                    shell_output("printf '%s' 'second br failure' >&2; exit 1")
                }
            },
            |_| {},
        );
    }))
    .expect_err("a failed retry must panic");

    assert_eq!(attempts.get(), 2);
    let message = panic_message(&panic);
    assert!(message.contains("retried once"));
    assert!(message.contains("second br failure"));
}

fn shell_output(script: &str) -> std::process::Output {
    Command::new("sh")
        .args(["-c", script])
        .output()
        .expect("sh must be available to exercise the br helper")
}

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> &str {
    panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .expect("panic must carry a string message")
}

#[cfg(unix)]
fn install_no_pr_gh_stub(fake_bin: &Path) {
    let fake_gh = fake_bin.join("gh");
    std::fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf 'no pull requests found for branch\n' >&2\nexit 1\n",
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_gh).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(fake_gh, permissions).unwrap();
}

#[cfg(unix)]
fn install_empty_herdr_stub(fake_bin: &Path) {
    let fake_herdr = fake_bin.join("herdr");
    std::fs::write(
        &fake_herdr,
        "#!/bin/sh\nif [ \"$1 $2\" = \"agent list\" ]; then\n  printf '%s\\n' '{\"result\":{\"agents\":[]}}'\nelse\n  printf 'unexpected herdr call: %s\\n' \"$*\" >&2\n  exit 2\nfi\n",
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_herdr).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(fake_herdr, permissions).unwrap();
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git must be on PATH");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn executable_file(candidate: &Path) -> bool {
    let Ok(metadata) = candidate.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn find_on_supplied_path(program: &str, path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path)
        .map(|dir| dir.join(program))
        .find(|candidate| executable_file(candidate))
}

fn program_is_on_path(program: &str, path: &OsStr) -> bool {
    find_on_supplied_path(program, path).is_some()
}

fn find_on_path(program: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| find_on_supplied_path(program, &path))
}

macro_rules! require_br {
    () => {
        if find_on_path("br").is_none() {
            eprintln!("skipping br-dependent test: br is not resolvable on PATH");
            return;
        }
    };
}

#[cfg(unix)]
#[test]
fn drain_enumerates_a_closed_bead_from_its_local_lane_branch() {
    require_br!();
    let ws = TempWorkspace::new("closed-lane-candidate");
    git(
        &ws.0,
        &[
            "-c",
            "user.name=Abacus Integration Test",
            "-c",
            "user.email=abacus@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "seed lane candidate",
        ],
    );
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=closed lane candidate"]);
    let ready = br(&ws.0, &["ready", "--json"]);
    let ready = parse_ready(&ready).unwrap();
    let bead = select_bead(&ready).unwrap();
    let bead_id = bead.id.clone();
    br(&ws.0, &["update", &bead_id, "--claim"]);
    br(&ws.0, &["close", &bead_id]);
    git(&ws.0, &["branch", &format!("lane/{bead_id}")]);

    let listed = br(&ws.0, &["list", "--json"]);
    assert!(
        !listed.contains(&bead_id),
        "real br must omit the closed fixture from its default list: {listed}"
    );

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    install_empty_herdr_stub(&fake_bin);
    install_no_pr_gh_stub(&fake_bin);
    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["drain", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains(&format!("completed: 1 [{bead_id}")),
        "the closed bead was not recovered from its local lane branch: {stdout}"
    );
}

#[test]
fn temp_workspace_allocations_with_the_same_tag_are_isolated() {
    let first = TempWorkspace::new("repeated-tag");
    let marker = first.0.join("first-workspace-marker");
    std::fs::write(&marker, "owned by the first workspace\n").unwrap();

    let second = TempWorkspace::new("repeated-tag");

    assert_ne!(
        first.0, second.0,
        "independent test processes can share a PID namespace and reuse tags"
    );
    assert!(
        marker.is_file(),
        "allocating a second workspace removed the first workspace"
    );
}

#[test]
fn real_br_stores_with_the_same_workspace_tag_do_not_overlap() {
    require_br!();
    let first = TempWorkspace::new("repeated-real-br-tag");
    br(&first.0, &["init", "--prefix", "it"]);
    br(&first.0, &["create", "--title=first workspace bead"]);

    let second = TempWorkspace::new("repeated-real-br-tag");
    br(&second.0, &["init", "--prefix", "it"]);
    br(&second.0, &["create", "--title=second workspace bead"]);

    let first_ready = parse_ready(&br(&first.0, &["ready", "--json"])).unwrap();
    let second_ready = parse_ready(&br(&second.0, &["ready", "--json"])).unwrap();
    assert_eq!(first_ready[0].title, "first workspace bead");
    assert_eq!(second_ready[0].title, "second workspace bead");
}

#[test]
fn ci_workflow_uses_the_manifest_toolchain_and_required_commands() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(repository.join("Cargo.toml")).unwrap();
    let rust_version = manifest
        .lines()
        .find_map(|line| {
            line.strip_prefix("rust-version = \"")
                .and_then(|value| value.strip_suffix('"'))
        })
        .expect("Cargo.toml must declare package.rust-version");
    let workflow = std::fs::read_to_string(repository.join(".github/workflows/ci.yml"))
        .expect("the standard CI workflow must exist");

    let rust_version_reader = r#"sed -n 's/^rust-version = "\([^"]*\)"$/\1/p' Cargo.toml"#;
    assert!(
        workflow.matches(rust_version_reader).count() == 3,
        "each CI job must read Cargo.toml's rust-version ({rust_version})"
    );
    assert!(
        workflow.contains("steps.rust-version.outputs.version"),
        "workflow must select the toolchain read from Cargo.toml"
    );
    assert!(
        workflow.contains("cargo test"),
        "workflow must run the portable test suite"
    );

    let test_job = workflow
        .split_once("  test:\n")
        .and_then(|(_, jobs)| jobs.split_once("\n  clippy:"))
        .map(|(test, _)| test)
        .expect("workflow must define the test job before clippy");
    let br_archive = "br-0.3.2-linux_amd64.tar.gz";
    for install_contract in [
        "- name: Install br",
        &format!(
            "https://github.com/Dicklesworthstone/beads_rust/releases/download/v0.3.2/{br_archive}"
        ),
        &format!(
            "e67c560e77e912490e44a65e3e9c13205210d171e729c5d801072ee508207288  $RUNNER_TEMP/{br_archive}"
        ),
        "sha256sum --check -",
        "install -m 0755 \"$RUNNER_TEMP/br\" \"$HOME/.cargo/bin/br\"",
        "br --version",
    ] {
        assert!(
            test_job.contains(install_contract),
            "test job must install checksum-verified br 0.3.2 before running the real-br tests; missing {install_contract:?}"
        );
    }
    assert!(
        test_job.find("- name: Install br") < test_job.find("- name: Test"),
        "test job must install br before cargo test"
    );
    let br_real_export = "export BR_REAL=\"$HOME/.cargo/bin/br\"";
    assert!(
        test_job.contains(br_real_export),
        "test job must point the shim at the checksum-verified br binary"
    );
    assert!(
        test_job.find(br_real_export) < test_job.find("cargo test"),
        "test job must export BR_REAL before cargo test"
    );
    assert!(
        workflow.contains("cargo clippy --all-targets --all-features -- -D warnings"),
        "workflow must deny clippy warnings"
    );
    assert!(
        workflow.contains("cargo fmt --check"),
        "workflow must check formatting"
    );

    let triggers = workflow
        .strip_prefix("name: CI\n\non:\n")
        .and_then(|rest| rest.split_once("\njobs:"))
        .map(|(triggers, _)| triggers)
        .expect("workflow must have a top-level on block before jobs");
    for trigger in ["pull_request:", "push:", "merge_group:"] {
        assert!(triggers.contains(trigger), "missing {trigger} trigger");
    }
    assert!(
        triggers.contains("      - main"),
        "push must target the default branch"
    );

    let jobs = workflow
        .split_once("\njobs:\n")
        .map(|(_, jobs)| jobs)
        .expect("workflow must define jobs");
    for job in ["test", "clippy", "fmt"] {
        assert!(
            jobs.lines().any(|line| line == format!("  {job}:")),
            "missing stable {job} job name"
        );
    }
}

#[cfg(unix)]
#[test]
fn br_presence_guard_follows_the_supplied_path() {
    let workspace = TempWorkspace::new("br-presence");
    let empty_bin = workspace.0.join("empty-bin");
    let populated_bin = workspace.0.join("populated-bin");
    std::fs::create_dir(&empty_bin).unwrap();
    std::fs::create_dir(&populated_bin).unwrap();

    assert!(!program_is_on_path("br", empty_bin.as_os_str()));

    let fake_br = populated_bin.join("br");
    std::fs::write(&fake_br, "#!/bin/sh\nexit 0\n").unwrap();
    let mut permissions = std::fs::metadata(&fake_br).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_br, permissions).unwrap();

    assert!(program_is_on_path("br", populated_bin.as_os_str()));
}

#[test]
fn ready_roundtrip_selects_highest_priority_bead() {
    require_br!();
    let ws = TempWorkspace::new("roundtrip");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(
        &ws.0,
        &["create", "--title=background chore", "--priority=2"],
    );
    br(&ws.0, &["create", "--title=urgent fix", "--priority=1"]);

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).expect("real br ready output must parse");
    assert_eq!(beads.len(), 2);

    let selected = select_bead(&beads).unwrap();
    assert_eq!(selected.title, "urgent fix");
    assert_eq!(selected.priority, 1);
    assert!(selected.id.starts_with("it-"), "id was {}", selected.id);
}

#[cfg(unix)]
#[test]
fn abacus_run_skips_an_operator_seat_bead() {
    require_br!();
    let ws = TempWorkspace::new("operator-seat");
    br(&ws.0, &["init", "--prefix", "it"]);
    let operator_id = br(
        &ws.0,
        &[
            "create",
            "--title=operator milestone",
            "--priority=0",
            "--silent",
        ],
    )
    .trim()
    .to_owned();
    br(
        &ws.0,
        &["label", "add", "--label=seat:operator", &operator_id],
    );
    let worker_id = br(
        &ws.0,
        &["create", "--title=worker task", "--priority=1", "--silent"],
    )
    .trim()
    .to_owned();

    let restricted_bin = ws.0.join("restricted-bin");
    std::fs::create_dir(&restricted_bin).unwrap();
    let real_br = find_on_path("br").expect("guarded by require_br");
    let br_calls = ws.0.join("br-calls");
    let br_wrapper = restricted_bin.join("br");
    std::fs::write(
        &br_wrapper,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nexec '{}' \"$@\"\n",
            br_calls.display(),
            real_br.display(),
        ),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&br_wrapper).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&br_wrapper, permissions).unwrap();
    symlink(
        find_on_path("git").expect("git must be on the base PATH"),
        restricted_bin.join("git"),
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", restricted_bin)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let br_calls = std::fs::read_to_string(&br_calls)
        .unwrap_or_else(|error| format!("<unavailable: {error}>"));
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {stdout}\nstderr: {stderr}\nbr calls:\n{br_calls}"
    );
    assert!(
        stdout.contains(&format!("selected {worker_id}")),
        "worker-seat bead was not selected; stdout: {stdout}\nstderr: {stderr}\nbr calls:\n{br_calls}"
    );
    let ready_calls: Vec<_> = br_calls
        .lines()
        .filter(|call| call.starts_with("ready "))
        .collect();
    assert_eq!(
        ready_calls,
        ["ready --json"],
        "selection must use labels from one ready query; br calls:\n{br_calls}"
    );
    assert_eq!(
        parse_bead_outcome(&br(&ws.0, &["show", &operator_id, "--json"])).unwrap(),
        BeadOutcome::NeverEngaged,
        "operator-seat bead must remain open"
    );
    assert_eq!(
        parse_bead_outcome(&br(&ws.0, &["show", &worker_id, "--json"])).unwrap(),
        BeadOutcome::Incomplete,
        "eligible worker bead must be claimed before lane creation"
    );
}

#[test]
fn abacus_run_on_empty_backlog_dispatches_nothing_and_exits_zero() {
    require_br!();
    let ws = TempWorkspace::new("emptyrun");
    br(&ws.0, &["init", "--prefix", "it"]);

    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("no ready beads"), "stdout: {stdout}");
}

#[cfg(unix)]
#[test]
fn abacus_drain_dispatches_every_ready_bead_then_exits_zero() {
    require_br!();
    let ws = TempWorkspace::new("drain-three-ready");
    br(&ws.0, &["init", "--prefix", "it"]);
    let bead_ids: Vec<_> = ["first worker", "second worker", "third worker"]
        .into_iter()
        .map(|title| {
            let title_arg = format!("--title={title}");
            br(&ws.0, &["create", &title_arg, "--silent"])
                .trim()
                .to_owned()
        })
        .collect();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let fake_herdr = fake_bin.join("herdr");
    let fake_gh = fake_bin.join("gh");
    let herdr_calls = ws.0.join("herdr-calls");
    let current_bead = ws.0.join("current-bead");
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               shift 2\n\
               bead_id=\n\
               while [ \"$#\" -gt 0 ]; do\n\
                 if [ \"$1\" = \"--label\" ]; then\n\
                   bead_id=$2\n\
                   break\n\
                 fi\n\
                 shift\n\
               done\n\
               printf '%s\\n' \"$bead_id\" > '{}'\n\
               printf '{{\"result\":{{\"type\":\"worktree_created\",\"workspace\":{{\"workspace_id\":\"workspace-%s\"}},\"root_pane\":{{\"pane_id\":\"pane-%s\"}},\"worktree\":{{\"path\":\"{}\",\"branch\":\"lane/%s\"}}}}}}\\n' \"$bead_id\" \"$bead_id\" \"$bead_id\"\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               IFS= read -r bead_id < '{}'\n\
               cd '{}'\n\
               br close \"$bead_id\"\n\
               printf 'worker settled\\n'\n\
             fi\n",
            herdr_calls.display(),
            current_bead.display(),
            ws.0.display(),
            current_bead.display(),
            ws.0.display(),
        ),
    )
    .unwrap();
    std::fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf 'no pull requests found for branch\\n' >&2\nexit 1\n",
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_herdr).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_herdr, permissions).unwrap();
    let mut permissions = std::fs::metadata(&fake_gh).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_gh, permissions).unwrap();

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["drain", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let calls = std::fs::read_to_string(herdr_calls).unwrap();
    let lane_open_calls: Vec<_> = calls
        .lines()
        .filter(|call| call.starts_with("worktree create"))
        .collect();
    assert_eq!(lane_open_calls.len(), 3, "Herdr calls:\n{calls}");
    for bead_id in bead_ids {
        assert!(
            lane_open_calls
                .iter()
                .any(|call| call.contains(&format!("--label {bead_id}"))),
            "no lane opened for {bead_id}; Herdr calls:\n{calls}"
        );
        assert_eq!(
            parse_bead_outcome(&br(&ws.0, &["show", &bead_id, "--json"])).unwrap(),
            BeadOutcome::Completed
        );
    }
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("no ready beads"),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[cfg(unix)]
#[test]
fn abacus_run_claims_the_selected_bead_before_opening_its_lane() {
    require_br!();
    let ws = TempWorkspace::new("claim-before-lane");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=claim before dispatch"]);

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let restricted_bin = ws.0.join("restricted-bin");
    std::fs::create_dir(&restricted_bin).unwrap();
    symlink(
        find_on_path("br").expect("guarded by require_br"),
        restricted_bin.join("br"),
    )
    .unwrap();
    symlink(
        find_on_path("git").expect("git must be on the base PATH"),
        restricted_bin.join("git"),
    )
    .unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", restricted_bin)
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("failed to spawn herdr"),
        "failure must occur while opening the lane; stderr: {stderr}"
    );
    assert!(
        stderr.contains("after "),
        "lane-leg failure must report elapsed time; stderr: {stderr}"
    );

    let state = br(&ws.0, &["show", &bead.id, "--json"]);
    assert_eq!(
        parse_bead_outcome(&state).unwrap(),
        BeadOutcome::Incomplete,
        "the selected bead must be claimed in the dispatching checkout"
    );
}

#[cfg(unix)]
#[test]
fn abacus_run_sanitizes_only_the_herdr_name_for_a_dotted_child_id() {
    require_br!();
    let ws = TempWorkspace::new("dotted-agent-name");
    br(&ws.0, &["init", "--prefix", "it"]);
    let parent_id = br(
        &ws.0,
        &["create", "--title=dispatch epic", "--type=epic", "--silent"],
    )
    .trim()
    .to_owned();
    let child_id = br(
        &ws.0,
        &[
            "create",
            "--title=dotted child dispatch",
            "--parent",
            &parent_id,
            "--priority=0",
            "--silent",
        ],
    )
    .trim()
    .to_owned();
    assert!(child_id.contains('.'), "child id was {child_id}");
    let agent_name = child_id.replace('.', "-");

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    install_no_pr_gh_stub(&fake_bin);
    let fake_herdr = fake_bin.join("herdr");
    let calls = ws.0.join("herdr-calls");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "dotted-workspace" },
            "root_pane": { "pane_id": "dotted-pane" },
            "worktree": {
                "path": ws.0,
                "branch": format!("lane/{child_id}")
            }
        }
    });
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{}'\n\
             elif [ \"$1 $2\" = \"agent start\" ]; then\n\
               case \"$3\" in\n\
                 [a-z]*) ;;\n\
                 *) printf '%s\\n' 'invalid_agent_name: name must start with a lowercase letter' >&2; exit 1 ;;\n\
               esac\n\
               case \"$3\" in\n\
                 *[!a-z0-9_-]*) printf '%s\\n' 'invalid_agent_name: unsupported character' >&2; exit 1 ;;\n\
               esac\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               if [ \"$3\" != '{}' ]; then\n\
                 printf '%s\\n' 'agent prompt used a different name' >&2\n\
                 exit 1\n\
               fi\n\
               cd '{}'\n\
               br update '{}' --claim\n\
               br close '{}'\n\
             fi\n",
            calls.display(),
            lane_json,
            agent_name,
            ws.0.display(),
            child_id,
            child_id,
        ),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_herdr).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_herdr, permissions).unwrap();

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("worker completed in "),
        "completed lane must report elapsed time; stdout: {stdout}"
    );
    let calls = std::fs::read_to_string(calls).unwrap();
    assert!(
        calls.lines().any(|call| {
            call.contains(&format!("--branch lane/{child_id}"))
                && call.contains(&format!("--label {child_id}"))
        }),
        "worktree identity changed:\n{calls}"
    );
    assert!(
        calls.lines().any(|call| {
            call == format!("agent start {agent_name} --kind codex --pane dotted-pane")
        }),
        "agent start did not use the sanitized name:\n{calls}"
    );
    assert!(
        calls.lines().any(|call| {
            call.starts_with(&format!("agent prompt {agent_name} "))
                && call.contains(&format!("br show {child_id}"))
                && call.contains(&format!("br close {child_id}"))
                && call.contains(&format!("git push -u origin lane/{child_id}"))
        }),
        "agent prompt lost sanitized routing or exact bead identity:\n{calls}"
    );
    let state = br(&ws.0, &["show", &child_id, "--json"]);
    assert_eq!(parse_bead_outcome(&state).unwrap(), BeadOutcome::Completed);
}

#[cfg(unix)]
#[test]
fn abacus_run_uses_the_discovered_default_branch_in_the_worker_prompt() {
    let ws = TempWorkspace::new("default-branch-prompt");
    let target_msrv = "1.82";
    std::fs::write(
        ws.0.join("Cargo.toml"),
        format!(
            "[package]\nname = \"target-fixture\"\nversion = \"0.0.0\"\nrust-version = \"{target_msrv}\"\n"
        ),
    )
    .unwrap();
    git(
        &ws.0,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/develop",
        ],
    );
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=dispatch to develop"]);

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    install_no_pr_gh_stub(&fake_bin);
    let fake_herdr = fake_bin.join("herdr");
    let calls = ws.0.join("herdr-calls");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "develop-workspace" },
            "root_pane": { "pane_id": "develop-pane" },
            "worktree": {
                "path": ws.0,
                "branch": format!("lane/{}", bead.id)
            }
        }
    });
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               cd '{}'\n\
               br close '{}'\n\
             fi\n",
            calls.display(),
            lane_json,
            ws.0.display(),
            bead.id,
        ),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_herdr).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_herdr, permissions).unwrap();

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let calls = std::fs::read_to_string(calls).unwrap();
    assert!(
        calls
            .lines()
            .any(|call| call.contains("gh pr create --base develop")
                && call.contains(&format!(
                    "RUSTUP_TOOLCHAIN={target_msrv} cargo clippy --all-targets --all-features -- -D warnings"
                ))),
        "worker prompt did not use the discovered default branch and target MSRV:\n{calls}"
    );
}

#[cfg(unix)]
#[test]
fn abacus_run_discovers_the_remote_default_without_a_local_origin_head() {
    let ws = TempWorkspace::new("remote-default-branch-fallback");
    git(
        &ws.0,
        &["symbolic-ref", "--delete", "refs/remotes/origin/HEAD"],
    );
    git(
        &ws.0,
        &[
            "-c",
            "user.name=Abacus Integration Test",
            "-c",
            "user.email=abacus@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "seed remote default",
        ],
    );
    let origin = ws.0.join("origin.git");
    let origin_arg = origin.to_str().unwrap();
    git(
        &ws.0,
        &["init", "--bare", "--initial-branch", "develop", origin_arg],
    );
    git(&ws.0, &["remote", "add", "origin", origin_arg]);
    git(&ws.0, &["push", "origin", "HEAD:refs/heads/develop"]);
    let local_head = Command::new("git")
        .args(["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"])
        .current_dir(&ws.0)
        .output()
        .unwrap();
    assert!(
        !local_head.status.success(),
        "fixture unexpectedly has refs/remotes/origin/HEAD"
    );

    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=dispatch via remote HEAD"]);

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    install_no_pr_gh_stub(&fake_bin);
    let fake_herdr = fake_bin.join("herdr");
    let calls = ws.0.join("herdr-calls");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "fallback-workspace" },
            "root_pane": { "pane_id": "fallback-pane" },
            "worktree": {
                "path": ws.0,
                "branch": format!("lane/{}", bead.id)
            }
        }
    });
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               cd '{}'\n\
               br close '{}'\n\
             fi\n",
            calls.display(),
            lane_json,
            ws.0.display(),
            bead.id,
        ),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_herdr).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_herdr, permissions).unwrap();

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let calls = std::fs::read_to_string(calls).unwrap();
    assert!(
        calls
            .lines()
            .any(|call| call.contains("gh pr create --base develop")),
        "worker prompt did not use the advertised remote default branch:\n{calls}"
    );
}

#[test]
fn abacus_run_names_both_default_branch_discovery_attempts_when_they_fail() {
    require_br!();
    let ws = TempWorkspace::new("default-branch-dual-failure");
    git(
        &ws.0,
        &["symbolic-ref", "--delete", "refs/remotes/origin/HEAD"],
    );
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=unresolvable default branch"]);

    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(out.status.code(), Some(1), "stderr: {stderr}");
    for attempted in [
        "git symbolic-ref --short refs/remotes/origin/HEAD",
        "git ls-remote --symref origin HEAD",
    ] {
        assert!(
            stderr.contains(attempted),
            "error did not name {attempted:?}: {stderr}"
        );
    }
}

#[test]
fn abacus_run_without_a_tracker_fails_with_brs_own_message() {
    require_br!();
    let ws = TempWorkspace::new("notracker");

    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr.contains("br"), "stderr: {stderr}");
}

#[cfg(unix)]
#[test]
fn abacus_run_probes_the_dispatching_store_instead_of_a_stale_lane_tracker() {
    require_br!();
    let ws = TempWorkspace::new("main-store-outcome");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=worker never engaged"]);

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let lane = ws.0.join("lane");
    let lane_tracker = lane.join(".beads");
    std::fs::create_dir_all(&lane_tracker).unwrap();
    for file in ["config.yaml", "issues.jsonl", "metadata.json"] {
        std::fs::copy(ws.0.join(".beads").join(file), lane_tracker.join(file)).unwrap();
    }

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let fake_herdr = fake_bin.join("herdr");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "fake-workspace" },
            "root_pane": { "pane_id": "fake-pane" },
            "worktree": {
                "path": lane,
                "branch": format!("lane/{}", bead.id)
            }
        }
    });
    std::fs::write(
        &fake_herdr,
        format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", lane_json),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_herdr).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_herdr, permissions).unwrap();
    install_no_pr_gh_stub(&fake_bin);

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("in_progress"), "stderr: {stderr}");
    assert!(
        !stderr.contains("never engaged"),
        "the stale lane tracker must not determine the outcome; stderr: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn abacus_run_reaps_a_clean_lane_without_force_after_the_worker_closes_its_bead() {
    require_br!();
    let ws = TempWorkspace::new("closed-outcome");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=worker completes"]);

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let fake_herdr = fake_bin.join("herdr");
    let fake_gh = fake_bin.join("gh");
    let calls = ws.0.join("herdr-calls");
    let gh_calls = ws.0.join("gh-calls");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "completed-workspace" },
            "root_pane": { "pane_id": "completed-pane" },
            "worktree": {
                "path": ws.0,
                "branch": format!("lane/{}", bead.id)
            }
        }
    });
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               cd '{}'\n\
               br update '{}' --claim\n\
               br close '{}'\n\
             fi\n",
            calls.display(),
            lane_json,
            ws.0.display(),
            bead.id,
            bead.id,
        ),
    )
    .unwrap();
    std::fs::write(
        &fake_gh,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nprintf 'no pull requests found for branch\\n' >&2\nexit 1\n",
            gh_calls.display()
        ),
    )
    .unwrap();
    std::fs::write(&gh_calls, "").unwrap();
    for fake_program in [&fake_herdr, &fake_gh] {
        let mut permissions = std::fs::metadata(fake_program).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(fake_program, permissions).unwrap();
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let calls = std::fs::read_to_string(calls).unwrap();
    let removal_calls: Vec<_> = calls
        .lines()
        .filter(|call| call.starts_with("worktree remove"))
        .collect();
    assert_eq!(
        removal_calls,
        ["worktree remove --workspace completed-workspace"],
        "Herdr calls:\n{calls}"
    );
    assert_eq!(
        std::fs::read_to_string(gh_calls).unwrap(),
        format!(
            "pr view lane/{} --json state,mergedAt,headRefOid,number,comments\n",
            bead.id
        ),
        "run must prove the completed lane has no PR before reaping"
    );
}

#[cfg(unix)]
#[test]
fn abacus_run_warns_and_forces_removal_when_a_completed_lane_is_dirty() {
    require_br!();
    let ws = TempWorkspace::new("dirty-completed-outcome");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=dirty worker completes"]);

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    install_no_pr_gh_stub(&fake_bin);
    let fake_herdr = fake_bin.join("herdr");
    let calls = ws.0.join("herdr-calls");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "dirty-workspace" },
            "root_pane": { "pane_id": "dirty-pane" },
            "worktree": {
                "path": ws.0,
                "branch": format!("lane/{}", bead.id)
            }
        }
    });
    let dirty_error = serde_json::json!({
        "id": "cli:worktree:remove",
        "error": {
            "code": "dirty_worktree_requires_force",
            "message": "worktree contains modified or untracked files; use --force to delete it"
        }
    });
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               cd '{}'\n\
               br update '{}' --claim\n\
               br close '{}'\n\
             elif [ \"$1 $2\" = \"worktree remove\" ] && [ \"$5\" != \"--force\" ]; then\n\
               printf '%s\\n' '{}' >&2\n\
               exit 1\n\
             fi\n",
            calls.display(),
            lane_json,
            ws.0.display(),
            bead.id,
            bead.id,
            dirty_error,
        ),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_herdr).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_herdr, permissions).unwrap();

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("WARNING"), "stderr: {stderr}");
    assert!(stderr.contains("dirty-workspace"), "stderr: {stderr}");
    assert!(
        stderr.contains("completed lane left uncommitted changes"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("protocol violation worth investigating"),
        "stderr: {stderr}"
    );

    let calls = std::fs::read_to_string(calls).unwrap();
    let removal_calls: Vec<_> = calls
        .lines()
        .filter(|call| call.starts_with("worktree remove"))
        .collect();
    assert_eq!(
        removal_calls,
        [
            "worktree remove --workspace dirty-workspace",
            "worktree remove --workspace dirty-workspace --force",
        ],
        "Herdr calls:\n{calls}"
    );
}

#[cfg(unix)]
#[test]
fn abacus_run_retries_once_when_the_first_agent_prompt_stalls() {
    require_br!();
    let ws = TempWorkspace::new("prompt-retry");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=worker survives prompt race"]);

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    install_no_pr_gh_stub(&fake_bin);
    let fake_herdr = fake_bin.join("herdr");
    let prompt_attempts = ws.0.join("prompt-attempts");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "retry-workspace" },
            "root_pane": { "pane_id": "retry-pane" },
            "worktree": {
                "path": ws.0,
                "branch": format!("lane/{}", bead.id)
            }
        }
    });
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               printf 'attempt\\n' >> '{}'\n\
               if [ \"$(wc -l < '{}')\" -eq 1 ]; then\n\
                 printf '%s\\n' 'agent prompt produced no observed state change within 5000 ms; status is idle and state_change_seq remained 1578.' >&2\n\
                 exit 1\n\
               fi\n\
               cd '{}'\n\
               br update '{}' --claim\n\
               br close '{}'\n\
             fi\n",
            lane_json,
            prompt_attempts.display(),
            prompt_attempts.display(),
            ws.0.display(),
            bead.id,
            bead.id,
        ),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_herdr).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_herdr, permissions).unwrap();

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(prompt_attempts).unwrap(),
        "attempt\nattempt\n"
    );
}

#[cfg(unix)]
#[test]
fn abacus_run_retries_a_transient_outcome_probe_before_reprompting_a_never_engaged_worker() {
    require_br!();
    let ws = TempWorkspace::new("transient-outcome-probe");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(
        &ws.0,
        &["create", "--title=worker survives transient outcome probe"],
    );

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    install_no_pr_gh_stub(&fake_bin);
    let fake_br = fake_bin.join("br");
    let fake_herdr = fake_bin.join("herdr");
    let prompt_attempts = ws.0.join("prompt-attempts");
    let probe_attempts = ws.0.join("probe-attempts");
    let prompt_settled = ws.0.join("prompt-settled");
    let failed_probe = ws.0.join("failed-probe");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "transient-probe-workspace" },
            "root_pane": { "pane_id": "transient-probe-pane" },
            "worktree": {
                "path": ws.0,
                "branch": format!("lane/{}", bead.id)
            }
        }
    });
    std::fs::write(
        &fake_br,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"show\" ] && [ -f '{}' ]; then\n\
               printf 'probe\\n' >> '{}'\n\
               if [ ! -f '{}' ]; then\n\
                 : > '{}'\n\
                 exit 7\n\
               fi\n\
             fi\n\
             exec '{}' \"$@\"\n",
            prompt_settled.display(),
            probe_attempts.display(),
            failed_probe.display(),
            failed_probe.display(),
            find_on_path("br").expect("guarded by require_br").display(),
        ),
    )
    .unwrap();
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               printf 'attempt\\n' >> '{}'\n\
               cd '{}'\n\
               if [ \"$(wc -l < '{}')\" -eq 1 ]; then\n\
                 br update '{}' --status open\n\
               else\n\
                 br close '{}'\n\
               fi\n\
               : > '{}'\n\
             fi\n",
            lane_json,
            prompt_attempts.display(),
            ws.0.display(),
            prompt_attempts.display(),
            bead.id,
            bead.id,
            prompt_settled.display(),
        ),
    )
    .unwrap();
    for fake_program in [&fake_br, &fake_herdr] {
        let mut permissions = std::fs::metadata(fake_program).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(fake_program, permissions).unwrap();
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(prompt_attempts).unwrap(),
        "attempt\nattempt\n",
        "a recovered NeverEngaged probe must feed the existing re-prompt path"
    );
    assert_eq!(
        std::fs::read_to_string(probe_attempts).unwrap(),
        "probe\nprobe\nprobe\n",
        "one failed probe, its retry, and the post-re-prompt probe are expected"
    );
    assert_eq!(
        parse_bead_outcome(&br(&ws.0, &["show", &bead.id, "--json"])).unwrap(),
        BeadOutcome::Completed
    );
}

#[cfg(unix)]
#[test]
fn abacus_run_reprompts_once_when_a_successful_prompt_never_engages_the_worker() {
    require_br!();
    let ws = TempWorkspace::new("never-engaged-retry-recovers");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(
        &ws.0,
        &["create", "--title=worker survives lost startup prompt"],
    );

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    install_no_pr_gh_stub(&fake_bin);
    let fake_herdr = fake_bin.join("herdr");
    let prompt_attempts = ws.0.join("prompt-attempts");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "never-engaged-retry-workspace" },
            "root_pane": { "pane_id": "never-engaged-retry-pane" },
            "worktree": {
                "path": ws.0,
                "branch": format!("lane/{}", bead.id)
            }
        }
    });
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               printf 'attempt\\n' >> '{}'\n\
               cd '{}'\n\
               if [ \"$(wc -l < '{}')\" -eq 1 ]; then\n\
                 br update '{}' --status open\n\
               else\n\
                 br close '{}'\n\
               fi\n\
             fi\n",
            lane_json,
            prompt_attempts.display(),
            ws.0.display(),
            prompt_attempts.display(),
            bead.id,
            bead.id,
        ),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_herdr).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_herdr, permissions).unwrap();

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(prompt_attempts).unwrap(),
        "attempt\nattempt\n"
    );
    assert_eq!(
        parse_bead_outcome(&br(&ws.0, &["show", &bead.id, "--json"])).unwrap(),
        BeadOutcome::Completed
    );
}

#[cfg(unix)]
#[test]
fn abacus_run_stops_after_a_second_never_engaged_outcome() {
    require_br!();
    let ws = TempWorkspace::new("never-engaged-retry-exhausted");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(
        &ws.0,
        &["create", "--title=worker loses both startup prompts"],
    );

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let fake_herdr = fake_bin.join("herdr");
    let prompt_attempts = ws.0.join("prompt-attempts");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "never-engaged-failure-workspace" },
            "root_pane": { "pane_id": "never-engaged-failure-pane" },
            "worktree": {
                "path": ws.0,
                "branch": format!("lane/{}", bead.id)
            }
        }
    });
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               printf 'attempt\\n' >> '{}'\n\
               cd '{}'\n\
               br update '{}' --status open\n\
             fi\n",
            lane_json,
            prompt_attempts.display(),
            ws.0.display(),
            bead.id,
        ),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_herdr).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_herdr, permissions).unwrap();
    install_no_pr_gh_stub(&fake_bin);

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("never engaged"), "stderr: {stderr}");
    assert_eq!(
        std::fs::read_to_string(prompt_attempts).unwrap(),
        "attempt\nattempt\n"
    );
}

#[cfg(unix)]
#[test]
fn abacus_drain_classifies_a_real_blocked_comment_and_continues() {
    require_br!();
    let ws = TempWorkspace::new("real-blocked-drain");
    br(&ws.0, &["init", "--prefix", "it"]);
    let first = br(
        &ws.0,
        &[
            "create",
            "--title=first worker blocks",
            "--priority=0",
            "--silent",
        ],
    )
    .trim()
    .to_owned();
    let second = br(
        &ws.0,
        &[
            "create",
            "--title=second worker completes",
            "--priority=1",
            "--silent",
        ],
    )
    .trim()
    .to_owned();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let fake_herdr = fake_bin.join("herdr");
    let fake_gh = fake_bin.join("gh");
    let current_bead = ws.0.join("current-bead");
    let herdr_calls = ws.0.join("herdr-calls");
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{calls}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               shift 2\n\
               while [ \"$#\" -gt 0 ]; do\n\
                 if [ \"$1\" = \"--label\" ]; then bead_id=$2; break; fi\n\
                 shift\n\
               done\n\
               printf '%s\\n' \"$bead_id\" > '{current_bead}'\n\
               printf '{{\"result\":{{\"type\":\"worktree_created\",\"workspace\":{{\"workspace_id\":\"workspace-%s\"}},\"root_pane\":{{\"pane_id\":\"pane-%s\"}},\"worktree\":{{\"path\":\"{root}\",\"branch\":\"lane/%s\"}}}}}}\\n' \"$bead_id\" \"$bead_id\" \"$bead_id\"\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               IFS= read -r bead_id < '{current_bead}'\n\
               cd '{root}'\n\
               if [ \"$bead_id\" = \"{first}\" ]; then\n\
                 br comments add \"$bead_id\" 'BLOCKED: fixture reason'\n\
               else\n\
                 br close \"$bead_id\"\n\
               fi\n\
             fi\n",
            calls = herdr_calls.display(),
            current_bead = current_bead.display(),
            root = ws.0.display(),
            first = first,
        ),
    )
    .unwrap();
    std::fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf 'no pull requests found for branch\\n' >&2\nexit 1\n",
    )
    .unwrap();
    for fake_program in [&fake_herdr, &fake_gh] {
        let mut permissions = std::fs::metadata(fake_program).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(fake_program, permissions).unwrap();
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["drain", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let first_state = br(&ws.0, &["show", &first, "--json"]);
    assert_eq!(
        parse_bead_outcome(&first_state).unwrap(),
        BeadOutcome::Blocked
    );
    assert!(
        first_state.contains("BLOCKED: fixture reason"),
        "blocked comment was not durable: {first_state}"
    );
    assert_eq!(
        parse_bead_outcome(&br(&ws.0, &["show", &second, "--json"])).unwrap(),
        BeadOutcome::Completed
    );
    assert!(stdout.contains(&format!("blocked: 1 [{first}")));
    assert!(stdout.contains(&format!("completed: 1 [{second}")));
}

#[cfg(unix)]
#[test]
fn abacus_run_classifies_a_superseded_blocked_comment_as_stalled() {
    require_br!();
    let ws = TempWorkspace::new("real-superseded-blocked");
    br(&ws.0, &["init", "--prefix", "it"]);
    let bead_id = br(
        &ws.0,
        &["create", "--title=superseded blocked worker", "--silent"],
    )
    .trim()
    .to_owned();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let fake_herdr = fake_bin.join("herdr");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "stalled-workspace" },
            "root_pane": { "pane_id": "stalled-pane" },
            "worktree": {
                "path": ws.0,
                "branch": format!("lane/{bead_id}")
            }
        }
    });
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{lane_json}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               cd '{root}'\n\
               br comments add '{bead_id}' 'BLOCKED: temporary fixture'\n\
               br comments add '{bead_id}' 'resuming'\n\
             fi\n",
            lane_json = lane_json,
            root = ws.0.display(),
            bead_id = bead_id,
        ),
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake_herdr).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake_herdr, permissions).unwrap();
    install_no_pr_gh_stub(&fake_bin);

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("stalled"), "stderr: {stderr}");
    assert_eq!(
        parse_bead_outcome(&br(&ws.0, &["show", &bead_id, "--json"])).unwrap(),
        BeadOutcome::Incomplete,
        "the later real-br comment must supersede BLOCKED"
    );
}

#[test]
fn abacus_without_a_command_prints_usage() {
    let out = Command::new(env!("CARGO_BIN_EXE_abacus")).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage"));

    for flag in ["--help", "-h"] {
        let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
            .arg(flag)
            .output()
            .unwrap();
        assert!(out.status.success(), "{flag} exited with {}", out.status);
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("abacus run"),
            "{flag} stdout: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(out.stderr.is_empty(), "{flag} wrote to stderr");
    }

    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .arg("bogus")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage"));
}

#[test]
fn dispatch_protocol_pushes_opens_pr_then_closes_without_lane_tracker_changes() {
    require_br!();
    let ws = TempWorkspace::new("closed-push");
    let origin = ws.0.join("origin.git");
    let tracker = ws.0.join("tracker");
    let lane = ws.0.join("lane");
    let origin_arg = origin.to_str().unwrap();
    let lane_arg = lane.to_str().unwrap();

    git(&ws.0, &["init", "--bare", origin_arg]);
    git(&ws.0, &["init", "-b", "lane/it-work", lane_arg]);
    std::fs::create_dir(&tracker).unwrap();
    br(&tracker, &["init", "--prefix", "it"]);
    br(&tracker, &["create", "--title=protocol regression"]);

    let json = br(&tracker, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead_id = &beads[0].id;
    let prompt = dispatch_prompt(bead_id, "lane/it-work", "main", None);
    let close = prompt.find(&format!("br close {bead_id}")).unwrap();
    let commit = prompt.find("commit all work").unwrap();
    let push = prompt.find("git push -u origin lane/it-work").unwrap();
    let pr = prompt.find("gh pr create --base main").unwrap();
    assert!(
        commit < push && push < pr && pr < close,
        "dispatch protocol is out of order: {prompt}"
    );
    assert!(!prompt.contains("--claim"), "prompt: {prompt}");
    assert!(!prompt.contains("git add .beads"), "prompt: {prompt}");
    assert!(
        prompt.contains(&format!("title containing `{bead_id}`")),
        "prompt: {prompt}"
    );
    assert!(prompt.contains("suite results"), "prompt: {prompt}");
    assert!(
        prompt.contains("red-first confirmation"),
        "prompt: {prompt}"
    );
    assert!(
        prompt.contains("treat that existing PR as success"),
        "prompt: {prompt}"
    );

    std::fs::write(lane.join("worker-change.txt"), "reviewable work\n").unwrap();
    git(&lane, &["add", "worker-change.txt"]);
    git(
        &lane,
        &[
            "-c",
            "user.name=Abacus Integration Test",
            "-c",
            "user.email=abacus@example.invalid",
            "commit",
            "-m",
            "implement worker change",
        ],
    );
    git(&lane, &["remote", "add", "origin", origin_arg]);

    br(&tracker, &["update", bead_id, "--claim"]);
    git(&lane, &["push", "-u", "origin", "lane/it-work"]);
    br(&tracker, &["close", bead_id]);

    assert!(git(&lane, &["status", "--porcelain"]).is_empty());
    let remote_files = git(
        &lane,
        &[
            "--git-dir",
            origin_arg,
            "ls-tree",
            "-r",
            "--name-only",
            "lane/it-work",
        ],
    );
    assert_eq!(remote_files, "worker-change.txt\n");
    assert_eq!(
        parse_bead_outcome(&br(&tracker, &["show", bead_id, "--json"])).unwrap(),
        BeadOutcome::Completed
    );
}

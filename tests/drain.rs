#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("abacus-{tag}-{}", std::process::id()));
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

#[test]
fn drain_reselects_after_a_lost_claim_and_dispatches_the_next_bead() {
    let workspace = TempDir::new("drain-claim-race");
    let fake_bin = workspace.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();

    let br_calls = workspace.0.join("br-calls");
    let herdr_calls = workspace.0.join("herdr-calls");
    let lost_claim = workspace.0.join("lost-claim");
    let completed = workspace.0.join("completed");

    let fake_br = fake_bin.join("br");
    std::fs::write(
        &fake_br,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n\
             if [ \"$1 $2\" = \"list --json\" ]; then\n\
               printf '{{\"issues\":[]}}\n'\n\
             elif [ \"$1\" = \"ready\" ]; then\n\
               if [ -f '{}' ]; then\n\
                 printf '[]\\n'\n\
               elif [ -f '{}' ]; then\n\
                 printf '[{{\"id\":\"it-second\",\"title\":\"second bead\",\"priority\":1,\"labels\":[]}}]\\n'\n\
               else\n\
                 printf '[{{\"id\":\"it-first\",\"title\":\"first bead\",\"priority\":0,\"labels\":[]}},{{\"id\":\"it-second\",\"title\":\"second bead\",\"priority\":1,\"labels\":[]}}]\\n'\n\
               fi\n\
             elif [ \"$1 $2 $3\" = \"update it-first --claim\" ]; then\n\
               : > '{}'\n\
               printf 'claim lost to another drain\\n' >&2\n\
               exit 1\n\
             elif [ \"$1 $2 $3\" = \"update it-second --claim\" ]; then\n\
               exit 0\n\
             elif [ \"$1 $2\" = \"show it-second\" ]; then\n\
               printf '[{{\"status\":\"closed\"}}]\\n'\n\
             else\n\
               printf 'unexpected br call: %s\\n' \"$*\" >&2\n\
               exit 2\n\
             fi\n",
            br_calls.display(),
            completed.display(),
            lost_claim.display(),
            lost_claim.display(),
        ),
    )
    .unwrap();

    let fake_herdr = fake_bin.join("herdr");
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{{\"result\":{{\"type\":\"worktree_created\",\"workspace\":{{\"workspace_id\":\"second-workspace\"}},\"root_pane\":{{\"pane_id\":\"second-pane\"}},\"worktree\":{{\"path\":\"{}\",\"branch\":\"lane/it-second\"}}}}}}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               : > '{}'\n\
               printf 'worker settled\\n'\n\
             fi\n",
            herdr_calls.display(),
            workspace.0.display(),
            completed.display(),
        ),
    )
    .unwrap();

    let fake_git = fake_bin.join("git");
    std::fs::write(
        &fake_git,
        "#!/bin/sh\nif [ \"$1 $2 $3\" = \"symbolic-ref --short refs/remotes/origin/HEAD\" ]; then\n  printf 'origin/main\\n'\nelse\n  printf 'unexpected git call: %s\\n' \"$*\" >&2\n  exit 2\nfi\n",
    )
    .unwrap();

    let fake_gh = fake_bin.join("gh");
    std::fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf 'no pull requests found for branch\\n' >&2\nexit 1\n",
    )
    .unwrap();

    for fake_program in [&fake_br, &fake_herdr, &fake_git, &fake_gh] {
        make_executable(fake_program);
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["drain", workspace.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let br_calls = std::fs::read_to_string(br_calls).unwrap();
    assert!(
        br_calls.contains("update it-first --claim"),
        "the first claim was not attempted:\n{br_calls}"
    );
    assert!(
        br_calls.contains("update it-second --claim"),
        "the second bead was not reselected:\n{br_calls}"
    );
    let herdr_calls = std::fs::read_to_string(herdr_calls).unwrap();
    let lane_open_calls: Vec<_> = herdr_calls
        .lines()
        .filter(|call| call.starts_with("worktree create"))
        .collect();
    assert_eq!(lane_open_calls.len(), 1, "Herdr calls:\n{herdr_calls}");
    assert!(
        lane_open_calls[0].contains("--branch lane/it-second")
            && lane_open_calls[0].contains("--label it-second"),
        "the wrong lane opened: {}",
        lane_open_calls[0]
    );
}

#[test]
fn drain_records_a_blocked_settle_and_continues_to_the_next_bead() {
    let workspace = TempDir::new("drain-blocked-continues");
    let fake_bin = workspace.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();

    let br_calls = workspace.0.join("br-calls");
    let herdr_calls = workspace.0.join("herdr-calls");
    let gh_calls = workspace.0.join("gh-calls");
    let active_bead = workspace.0.join("active-bead");
    let blocked = workspace.0.join("blocked");
    let completed = workspace.0.join("completed");

    let fake_br = fake_bin.join("br");
    std::fs::write(
        &fake_br,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{br_calls}'\n\
             if [ \"$1 $2\" = \"list --json\" ]; then\n\
               printf '{{\"issues\":[]}}\n'\n\
             elif [ \"$1\" = \"ready\" ]; then\n\
               if [ -f '{completed}' ]; then\n\
                 printf '[]\\n'\n\
               elif [ -f '{blocked}' ]; then\n\
                 printf '[{{\"id\":\"it-second\",\"title\":\"second bead\",\"priority\":1,\"labels\":[]}}]\\n'\n\
               else\n\
                 printf '[{{\"id\":\"it-first\",\"title\":\"first bead\",\"priority\":0,\"labels\":[]}},{{\"id\":\"it-second\",\"title\":\"second bead\",\"priority\":1,\"labels\":[]}}]\\n'\n\
               fi\n\
             elif [ \"$1\" = \"update\" ] && [ \"$3\" = \"--claim\" ]; then\n\
               printf '%s\\n' \"$2\" > '{active_bead}'\n\
             elif [ \"$1 $2\" = \"show it-first\" ]; then\n\
               printf '[{{\"status\":\"in_progress\",\"comments\":[{{\"id\":1,\"text\":\"BLOCKED: fixture reason\"}}]}}]\\n'\n\
             elif [ \"$1 $2\" = \"show it-second\" ]; then\n\
               printf '[{{\"status\":\"closed\"}}]\\n'\n\
             else\n\
               printf 'unexpected br call: %s\\n' \"$*\" >&2\n\
               exit 2\n\
             fi\n",
            br_calls = br_calls.display(),
            completed = completed.display(),
            blocked = blocked.display(),
            active_bead = active_bead.display(),
        ),
    )
    .unwrap();

    let fake_herdr = fake_bin.join("herdr");
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{herdr_calls}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               IFS= read -r bead_id < '{active_bead}'\n\
               printf '{{\"result\":{{\"type\":\"worktree_created\",\"workspace\":{{\"workspace_id\":\"workspace-%s\"}},\"root_pane\":{{\"pane_id\":\"pane-%s\"}},\"worktree\":{{\"path\":\"{root}\",\"branch\":\"lane/%s\"}}}}}}\\n' \"$bead_id\" \"$bead_id\" \"$bead_id\"\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               IFS= read -r bead_id < '{active_bead}'\n\
               if [ \"$bead_id\" = \"it-first\" ]; then\n\
                 : > '{blocked}'\n\
               else\n\
                 : > '{completed}'\n\
               fi\n\
               printf 'worker settled\\n'\n\
             fi\n",
            herdr_calls = herdr_calls.display(),
            active_bead = active_bead.display(),
            root = workspace.0.display(),
            blocked = blocked.display(),
            completed = completed.display(),
        ),
    )
    .unwrap();

    let fake_git = fake_bin.join("git");
    std::fs::write(
        &fake_git,
        "#!/bin/sh\nif [ \"$1 $2 $3\" = \"symbolic-ref --short refs/remotes/origin/HEAD\" ]; then\n  printf 'origin/main\\n'\nelse\n  printf 'unexpected git call: %s\\n' \"$*\" >&2\n  exit 2\nfi\n",
    )
    .unwrap();

    let fake_gh = fake_bin.join("gh");
    std::fs::write(
        &fake_gh,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nprintf 'no pull requests found for branch\\n' >&2\nexit 1\n",
            gh_calls.display(),
        ),
    )
    .unwrap();
    std::fs::write(&gh_calls, "").unwrap();

    for fake_program in [&fake_br, &fake_herdr, &fake_git, &fake_gh] {
        make_executable(fake_program);
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["drain", workspace.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = std::fs::read_to_string(herdr_calls).unwrap();
    assert_eq!(
        calls.lines().next(),
        Some("agent list"),
        "each drain iteration must sweep before dispatch:\n{calls}"
    );
    assert_eq!(
        calls
            .lines()
            .filter(|call| call.starts_with("worktree create"))
            .count(),
        2,
        "both lanes must open:\n{calls}"
    );
    assert!(stdout.contains("blocked: 1 [it-first"), "stdout: {stdout}");
    assert!(
        stdout.contains("completed: 1 [it-second"),
        "stdout: {stdout}"
    );
    assert!(
        !calls.lines().any(|call| call.ends_with(" --force")),
        "a blocked lane must never be force-reaped:\n{calls}"
    );
    let gh_calls = std::fs::read_to_string(gh_calls).unwrap();
    assert!(
        !gh_calls.lines().any(|call| call.contains("lane/it-first")),
        "Blocked must stay absorbing and PR-unprobed across this multi-sweep drain:\n{gh_calls}"
    );
}

#[test]
fn drain_records_awaiting_review_and_exits_when_nothing_is_actionable() {
    let workspace = TempDir::new("drain-awaiting-review");
    let fake_bin = workspace.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let settled = workspace.0.join("settled");
    let herdr_calls = workspace.0.join("herdr-calls");

    let fake_br = fake_bin.join("br");
    std::fs::write(
        &fake_br,
        format!(
            "#!/bin/sh\n\
             if [ \"$1 $2\" = \"list --json\" ]; then\n\
               printf '{{\"issues\":[]}}\n'\n\
             elif [ \"$1\" = \"ready\" ]; then\n\
               if [ -f '{settled}' ]; then printf '[]\\n'; else printf '[{{\"id\":\"it-review\",\"title\":\"review bead\",\"priority\":0,\"labels\":[]}}]\\n'; fi\n\
             elif [ \"$1 $2 $3\" = \"update it-review --claim\" ]; then\n\
               exit 0\n\
             elif [ \"$1 $2\" = \"show it-review\" ]; then\n\
               printf '[{{\"status\":\"closed\"}}]\\n'\n\
             else\n\
               printf 'unexpected br call: %s\\n' \"$*\" >&2; exit 2\n\
             fi\n",
            settled = settled.display(),
        ),
    )
    .unwrap();

    let fake_herdr = fake_bin.join("herdr");
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{herdr_calls}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{{\"result\":{{\"type\":\"worktree_created\",\"workspace\":{{\"workspace_id\":\"review-workspace\"}},\"root_pane\":{{\"pane_id\":\"review-pane\"}},\"worktree\":{{\"path\":\"{root}\",\"branch\":\"lane/it-review\"}}}}}}'\n\
             elif [ \"$1 $2\" = \"workspace create\" ]; then\n\
               printf '%s\\n' '{{\"result\":{{\"type\":\"workspace_created\",\"workspace\":{{\"workspace_id\":\"reviewer-workspace\"}},\"root_pane\":{{\"pane_id\":\"reviewer-pane\"}}}}}}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               : > '{settled}'\n\
               printf 'worker settled\\n'\n\
             fi\n",
            herdr_calls = herdr_calls.display(),
            root = workspace.0.display(),
            settled = settled.display(),
        ),
    )
    .unwrap();

    let fake_git = fake_bin.join("git");
    std::fs::write(
        &fake_git,
        "#!/bin/sh\nif [ \"$1 $2 $3\" = \"symbolic-ref --short refs/remotes/origin/HEAD\" ]; then printf 'origin/main\\n'; else exit 2; fi\n",
    )
    .unwrap();
    let fake_gh = fake_bin.join("gh");
    std::fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$5\" = \"number\" ]; then printf '42\\n'; else printf '%s\\n' '{\"state\":\"OPEN\",\"mergedAt\":null,\"headRefOid\":\"abc123\"}'; fi\n",
    )
    .unwrap();
    for fake_program in [&fake_br, &fake_herdr, &fake_git, &fake_gh] {
        make_executable(fake_program);
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["drain", workspace.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("awaiting-review: 1 [it-review"),
        "stdout: {stdout}"
    );
    let calls = std::fs::read_to_string(herdr_calls).unwrap();
    assert!(
        !calls
            .lines()
            .any(|call| call.starts_with("worktree remove")),
        "an awaiting-review lane must remain warm:\n{calls}"
    );
}

#[test]
fn run_classifies_closed_open_pr_as_awaiting_review_and_keeps_lane_warm() {
    let workspace = TempDir::new("run-awaiting-review");
    let fake_bin = workspace.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let settled = workspace.0.join("settled");
    let herdr_calls = workspace.0.join("herdr-calls");
    let gh_calls = workspace.0.join("gh-calls");

    let fake_br = fake_bin.join("br");
    std::fs::write(
        &fake_br,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"ready\" ]; then\n\
               if [ -f '{settled}' ]; then printf '[]\\n'; else printf '[{{\"id\":\"it-run-review\",\"title\":\"run review bead\",\"priority\":0,\"labels\":[]}}]\\n'; fi\n\
             elif [ \"$1 $2 $3\" = \"update it-run-review --claim\" ]; then\n\
               exit 0\n\
             elif [ \"$1 $2\" = \"show it-run-review\" ]; then\n\
               printf '[{{\"status\":\"closed\"}}]\\n'\n\
             else\n\
               printf 'unexpected br call: %s\\n' \"$*\" >&2; exit 2\n\
             fi\n",
            settled = settled.display(),
        ),
    )
    .unwrap();

    let fake_herdr = fake_bin.join("herdr");
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{herdr_calls}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{{\"result\":{{\"type\":\"worktree_created\",\"workspace\":{{\"workspace_id\":\"run-review-workspace\"}},\"root_pane\":{{\"pane_id\":\"run-review-pane\"}},\"worktree\":{{\"path\":\"{root}\",\"branch\":\"lane/it-run-review\"}}}}}}'\n\
             elif [ \"$1 $2\" = \"workspace create\" ]; then\n\
               printf '%s\\n' '{{\"result\":{{\"type\":\"workspace_created\",\"workspace\":{{\"workspace_id\":\"run-reviewer-workspace\"}},\"root_pane\":{{\"pane_id\":\"run-reviewer-pane\"}}}}}}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               : > '{settled}'\n\
               printf 'worker settled\\n'\n\
             fi\n",
            herdr_calls = herdr_calls.display(),
            root = workspace.0.display(),
            settled = settled.display(),
        ),
    )
    .unwrap();

    let fake_git = fake_bin.join("git");
    std::fs::write(
        &fake_git,
        "#!/bin/sh\nif [ \"$1 $2 $3\" = \"symbolic-ref --short refs/remotes/origin/HEAD\" ]; then printf 'origin/main\\n'; else exit 2; fi\n",
    )
    .unwrap();
    let fake_gh = fake_bin.join("gh");
    std::fs::write(
        &fake_gh,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"$5\" = \"number\" ]; then printf '42\\n'; else printf '%s\\n' '{{\"state\":\"OPEN\",\"mergedAt\":null,\"headRefOid\":\"review-head\"}}'; fi\n",
            gh_calls.display(),
        ),
    )
    .unwrap();
    std::fs::write(&gh_calls, "").unwrap();
    for fake_program in [&fake_br, &fake_herdr, &fake_git, &fake_gh] {
        make_executable(fake_program);
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", workspace.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("lane is awaiting-review"),
        "stdout: {stdout}"
    );
    let calls = std::fs::read_to_string(herdr_calls).unwrap();
    assert!(
        !calls
            .lines()
            .any(|call| call.starts_with("worktree remove")),
        "an awaiting-review run lane must remain warm:\n{calls}"
    );
    assert!(
        std::fs::read_to_string(gh_calls)
            .unwrap()
            .contains("pr view lane/it-run-review --json state,mergedAt,headRefOid"),
        "run must probe the lane PR before classifying the settle"
    );
}

#[test]
fn restart_sweep_reports_absent_in_progress_agent_as_stalled_and_continues() {
    let workspace = TempDir::new("restart-stalled-continues");
    let fake_bin = workspace.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let completed = workspace.0.join("completed");
    let herdr_calls = workspace.0.join("herdr-calls");

    let fake_br = fake_bin.join("br");
    std::fs::write(
        &fake_br,
        format!(
            "#!/bin/sh\n\
             if [ \"$1 $2\" = \"list --json\" ]; then\n\
               printf '{{\"issues\":[{{\"id\":\"it-stalled\",\"status\":\"in_progress\"}}]}}\\n'\n\
             elif [ \"$1\" = \"ready\" ]; then\n\
               if [ -f '{completed}' ]; then printf '[]\\n'; else printf '[{{\"id\":\"it-next\",\"title\":\"next bead\",\"priority\":0,\"labels\":[]}}]\\n'; fi\n\
             elif [ \"$1 $2 $3\" = \"update it-next --claim\" ]; then\n\
               exit 0\n\
             elif [ \"$1 $2\" = \"show it-stalled\" ]; then\n\
               printf '[{{\"status\":\"in_progress\",\"comments\":[]}}]\\n'\n\
             elif [ \"$1 $2\" = \"show it-next\" ]; then\n\
               printf '[{{\"status\":\"closed\"}}]\\n'\n\
             else\n\
               printf 'unexpected br call: %s\\n' \"$*\" >&2; exit 2\n\
             fi\n",
            completed = completed.display(),
        ),
    )
    .unwrap();

    let fake_herdr = fake_bin.join("herdr");
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{herdr_calls}'\n\
             if [ \"$1 $2\" = \"agent list\" ]; then\n\
               printf '%s\\n' '{{\"result\":{{\"agents\":[]}}}}'\n\
             elif [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{{\"result\":{{\"type\":\"worktree_created\",\"workspace\":{{\"workspace_id\":\"next-workspace\"}},\"root_pane\":{{\"pane_id\":\"next-pane\"}},\"worktree\":{{\"path\":\"{root}\",\"branch\":\"lane/it-next\"}}}}}}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               : > '{completed}'\n\
               printf 'worker settled\\n'\n\
             fi\n",
            herdr_calls = herdr_calls.display(),
            root = workspace.0.display(),
            completed = completed.display(),
        ),
    )
    .unwrap();

    let fake_git = fake_bin.join("git");
    std::fs::write(
        &fake_git,
        "#!/bin/sh\nif [ \"$1 $2 $3\" = \"symbolic-ref --short refs/remotes/origin/HEAD\" ]; then printf 'origin/main\\n'; else exit 2; fi\n",
    )
    .unwrap();
    let fake_gh = fake_bin.join("gh");
    std::fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf 'no pull requests found for branch\\n' >&2\nexit 1\n",
    )
    .unwrap();
    for fake_program in [&fake_br, &fake_herdr, &fake_git, &fake_gh] {
        make_executable(fake_program);
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["drain", workspace.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("stalled: 1 [it-stalled"),
        "restart failed to reconstruct the absent-agent lane as Stalled: {stdout}"
    );
    assert!(
        stdout.contains("completed: 1 [it-next"),
        "drain did not continue to dispatch the next ready bead: {stdout}"
    );
    let calls = std::fs::read_to_string(herdr_calls).unwrap();
    assert!(
        calls
            .lines()
            .any(|call| call.starts_with("worktree create") && call.contains("lane/it-next")),
        "the ready bead was not dispatched after the stalled reconstruction:\n{calls}"
    );
}

fn run_absent_closed_pr_sweep(
    tag: &str,
    bead_id: &str,
    pull_request_json: &str,
    force_resweep: bool,
) -> (std::process::Output, String, String) {
    let workspace = TempDir::new(tag);
    let fake_bin = workspace.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let herdr_calls = workspace.0.join("herdr-calls");
    let gh_calls = workspace.0.join("gh-calls");
    let ready_json = if force_resweep {
        r#"[{"id":"it-lost","title":"lost claim","priority":0,"labels":[]}]"#
    } else {
        "[]"
    };

    let fake_br = fake_bin.join("br");
    std::fs::write(
        &fake_br,
        format!(
            "#!/bin/sh\n\
             if [ \"$1 $2\" = \"list --json\" ]; then\n\
               printf '{{\"issues\":[{{\"id\":\"{bead_id}\",\"status\":\"closed\"}}]}}\n'\n\
             elif [ \"$1\" = \"ready\" ]; then\n\
               printf '%s\n' '{ready_json}'\n\
             elif [ \"$1 $2 $3\" = \"update it-lost --claim\" ]; then\n\
               printf 'fixture claim loss\n' >&2; exit 1\n\
             elif [ \"$1 $2\" = \"show {bead_id}\" ]; then\n\
               printf '[{{\"status\":\"closed\"}}]\n'\n\
             else\n\
               printf 'unexpected br call: %s\n' \"$*\" >&2; exit 2\n\
             fi\n",
        ),
    )
    .unwrap();

    let fake_herdr = fake_bin.join("herdr");
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\n' \"$*\" >> '{}'\n\
             if [ \"$1 $2\" = \"agent list\" ]; then\n\
               printf '%s\n' '{{\"result\":{{\"agents\":[]}}}}'\n\
             elif [ \"$1 $2\" = \"workspace create\" ]; then\n\
               printf '%s\n' '{{\"result\":{{\"type\":\"workspace_created\",\"workspace\":{{\"workspace_id\":\"restart-reviewer-workspace\"}},\"root_pane\":{{\"pane_id\":\"restart-reviewer-pane\"}}}}}}'\n\
             fi\n",
            herdr_calls.display(),
        ),
    )
    .unwrap();

    let fake_gh = fake_bin.join("gh");
    std::fs::write(
        &fake_gh,
        format!(
            "#!/bin/sh\nprintf '%s\n' \"$*\" >> '{}'\nif [ \"$5\" = \"number\" ]; then printf '42\n'; else printf '%s\n' '{}'; fi\n",
            gh_calls.display(),
            pull_request_json,
        ),
    )
    .unwrap();
    let fake_git = fake_bin.join("git");
    std::fs::write(
        &fake_git,
        "#!/bin/sh\nif [ \"$1 $2 $3\" = \"symbolic-ref --short refs/remotes/origin/HEAD\" ]; then printf 'origin/main\\n'; else exit 2; fi\n",
    )
    .unwrap();
    std::fs::write(&gh_calls, "").unwrap();
    for fake_program in [&fake_br, &fake_herdr, &fake_gh, &fake_git] {
        make_executable(fake_program);
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["drain", workspace.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();
    (
        output,
        std::fs::read_to_string(herdr_calls).unwrap(),
        std::fs::read_to_string(gh_calls).unwrap(),
    )
}

#[test]
fn restart_sweep_reports_absent_closed_open_pr_as_awaiting_review() {
    let bead_id = "it-closed-review";
    let (output, herdr_calls, gh_calls) = run_absent_closed_pr_sweep(
        "restart-closed-review",
        bead_id,
        r#"{"state":"OPEN","mergedAt":null,"headRefOid":"review-head"}"#,
        false,
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("awaiting-review: 1 [it-closed-review"),
        "restart failed to reconstruct the absent closed lane as AwaitingReview: {stdout}"
    );
    assert_eq!(
        gh_calls,
        format!(
            "pr view lane/{bead_id} --json state,mergedAt,headRefOid\npr view lane/{bead_id} --json number --jq .number\n"
        ),
        "the live open PR and its review target number are probed once"
    );
    assert!(
        !herdr_calls
            .lines()
            .any(|call| call.starts_with("worktree remove")),
        "AwaitingReview must remain warm:\n{herdr_calls}"
    );
}

#[test]
fn sweep_launches_one_ephemeral_reviewer_for_a_newly_awaiting_review_lane() {
    let bead_id = "it-review";
    let workspace = TempDir::new("drain-review-launch");
    let fake_bin = workspace.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    std::fs::write(workspace.0.join("AGENTS.md"), "review fixture authority\n").unwrap();
    let herdr_calls = workspace.0.join("herdr-calls");

    let fake_br = fake_bin.join("br");
    std::fs::write(
        &fake_br,
        format!(
            "#!/bin/sh\n\
             if [ \"$1 $2\" = \"list --json\" ]; then\n\
               printf '{{\"issues\":[{{\"id\":\"{bead_id}\",\"status\":\"closed\"}}]}}\\n'\n\
             elif [ \"$1\" = \"ready\" ]; then\n\
               printf '%s\\n' '[{{\"id\":\"it-lost\",\"title\":\"lost claim\",\"priority\":0,\"labels\":[]}}]'\n\
             elif [ \"$1 $2 $3\" = \"update it-lost --claim\" ]; then\n\
               printf 'fixture claim loss\\n' >&2; exit 1\n\
             elif [ \"$1 $2\" = \"show {bead_id}\" ]; then\n\
               printf '[{{\"id\":\"{bead_id}\",\"status\":\"closed\",\"description\":\"Review the target implementation.\",\"comments\":[{{\"id\":1,\"text\":\"Preserve the authority trail.\"}}]}}]\\n'\n\
             else\n\
               printf 'unexpected br call: %s\\n' \"$*\" >&2; exit 2\n\
             fi\n",
        ),
    )
    .unwrap();

    let fake_herdr = fake_bin.join("herdr");
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{calls}'\n\
             if [ \"$1 $2\" = \"agent list\" ]; then\n\
               printf '%s\\n' '{{\"result\":{{\"agents\":[]}}}}'\n\
             elif [ \"$1 $2\" = \"workspace create\" ]; then\n\
               printf '%s\\n' '{{\"result\":{{\"type\":\"workspace_created\",\"workspace\":{{\"workspace_id\":\"review-workspace\"}},\"root_pane\":{{\"pane_id\":\"review-pane\"}}}}}}'\n\
             elif [ \"$1 $2\" = \"agent start\" ]; then\n\
               exit 0\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               if [ ! -f \"$4\" ]; then printf 'missing brief: %s\\n' \"$4\" >&2; exit 3; fi\n\
               printf 'reviewer settled\\n'\n\
             else\n\
               printf 'unexpected herdr call: %s\\n' \"$*\" >&2; exit 2\n\
             fi\n",
            calls = herdr_calls.display(),
        ),
    )
    .unwrap();

    let fake_gh = fake_bin.join("gh");
    std::fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1 $2 $3\" = \"pr view lane/it-review\" ] && [ \"$5\" = \"state,mergedAt,headRefOid\" ]; then\n  printf '%s\\n' '{\"state\":\"OPEN\",\"mergedAt\":null,\"headRefOid\":\"review-head\"}'\nelif [ \"$1 $2 $3\" = \"pr view lane/it-review\" ] && [ \"$5\" = \"number\" ]; then\n  printf '42\\n'\nelse\n  printf 'unexpected gh call: %s\\n' \"$*\" >&2\n  exit 2\nfi\n",
    )
    .unwrap();
    let fake_git = fake_bin.join("git");
    std::fs::write(
        &fake_git,
        "#!/bin/sh\nif [ \"$1 $2 $3\" = \"symbolic-ref --short refs/remotes/origin/HEAD\" ]; then printf 'origin/main\\n'; else exit 2; fi\n",
    )
    .unwrap();
    for fake_program in [&fake_br, &fake_herdr, &fake_gh, &fake_git] {
        make_executable(fake_program);
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["drain", workspace.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let calls = std::fs::read_to_string(herdr_calls).unwrap();
    let workspace_creates: Vec<_> = calls
        .lines()
        .filter(|call| call.starts_with("workspace create"))
        .collect();
    assert_eq!(workspace_creates.len(), 1, "Herdr calls:\n{calls}");
    assert!(
        workspace_creates[0].contains(&format!("--cwd {}", workspace.0.display()))
            && workspace_creates[0].contains("--no-focus"),
        "reviewer did not get a dedicated workspace on the main checkout: {calls}"
    );
    assert!(
        !calls
            .lines()
            .any(|call| call.starts_with("worktree create")),
        "review launch must not create a worktree:\n{calls}"
    );
    assert_eq!(
        calls
            .lines()
            .filter(|call| call.starts_with("agent start rev-it-review-c1"))
            .count(),
        1,
        "reviewer must start once:\n{calls}"
    );
    assert!(
        calls.contains("agent start rev-it-review-c1 --kind codex --pane review-pane"),
        "reviewer start lacked the codex kind or dedicated pane: {calls}"
    );
    let prompts: Vec<_> = calls
        .lines()
        .filter(|call| call.starts_with("agent prompt rev-it-review-c1"))
        .collect();
    assert_eq!(prompts.len(), 1, "reviewer must be prompted once:\n{calls}");
    let prompt_parts: Vec<_> = prompts[0].split_whitespace().collect();
    assert_eq!(prompt_parts.last(), Some(&"--wait"));
    let brief = Path::new(prompt_parts[3]);
    assert!(
        brief.exists(),
        "review prompt path did not exist: {}",
        brief.display()
    );
    assert!(brief.starts_with(workspace.0.join("target/abacus-tmp/reviews")));
    assert!(
        !calls.contains("agent wait --until idle"),
        "the measured-broken wait form reappeared:\n{calls}"
    );
}

#[test]
fn restart_sweep_reports_absent_closed_merged_pr_as_merged() {
    let bead_id = "it-closed-merged";
    let (output, herdr_calls, gh_calls) = run_absent_closed_pr_sweep(
        "restart-closed-merged",
        bead_id,
        r#"{"state":"MERGED","mergedAt":"2026-08-19T18:00:00Z","headRefOid":"merged-head"}"#,
        true,
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("merged: 1 [it-closed-merged"),
        "restart failed to reconstruct the absent closed lane as Merged: {stdout}"
    );
    assert_eq!(
        gh_calls,
        format!("pr view lane/{bead_id} --json state,mergedAt,headRefOid\n"),
        "Merged becomes absorbing after its first handled probe"
    );
    assert!(
        !herdr_calls
            .lines()
            .any(|call| call.starts_with("worktree remove")),
        "there is no recorded workspace to reap:\n{herdr_calls}"
    );
}

fn run_in_progress_merged_pr_sweep(
    tag: &str,
    bead_id: &str,
    agent_status: Option<&str>,
) -> (std::process::Output, String, String) {
    let workspace = TempDir::new(tag);
    let fake_bin = workspace.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let herdr_calls = workspace.0.join("herdr-calls");
    let gh_calls = workspace.0.join("gh-calls");
    let agent_listed = workspace.0.join("agent-listed");

    let fake_br = fake_bin.join("br");
    std::fs::write(
        &fake_br,
        format!(
            "#!/bin/sh\n\
             if [ \"$1 $2\" = \"list --json\" ]; then\n\
               printf '{{\"issues\":[{{\"id\":\"{bead_id}\",\"status\":\"in_progress\"}}]}}\n'\n\
             elif [ \"$1\" = \"ready\" ]; then\n\
               printf '%s\n' '[{{\"id\":\"it-lost\",\"title\":\"lost claim\",\"priority\":0,\"labels\":[]}}]'\n\
             elif [ \"$1 $2 $3\" = \"update it-lost --claim\" ]; then\n\
               printf 'fixture claim loss\n' >&2; exit 1\n\
             elif [ \"$1 $2\" = \"show {bead_id}\" ]; then\n\
               printf '[{{\"status\":\"in_progress\",\"comments\":[]}}]\n'\n\
             else\n\
               printf 'unexpected br call: %s\n' \"$*\" >&2; exit 2\n\
             fi\n",
        ),
    )
    .unwrap();

    let agent_json = match agent_status {
        None => r#"{"result":{"agents":[]}}"#.to_owned(),
        Some(status) => format!(
            r#"{{"result":{{"agents":[{{"name":"{bead_id}","agent_status":"{status}","cwd":"{}","workspace_id":"workspace-{bead_id}","pane_id":"pane-{bead_id}"}}]}}}}"#,
            workspace.0.display(),
        ),
    };
    let done_agent_json = agent_status.map(|_| {
        format!(
            r#"{{"result":{{"agents":[{{"name":"{bead_id}","agent_status":"done","cwd":"{}","workspace_id":"workspace-{bead_id}","pane_id":"pane-{bead_id}"}}]}}}}"#,
            workspace.0.display(),
        )
    });
    let fake_herdr = fake_bin.join("herdr");
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\n' \"$*\" >> '{herdr_calls}'\n\
             if [ \"$1 $2\" = \"agent list\" ]; then\n\
               if [ -f '{agent_listed}' ]; then printf '%s\n' '{done_agent_json}'; else : > '{agent_listed}'; printf '%s\n' '{agent_json}'; fi\n\
             elif [ \"$1 $2\" = \"worktree remove\" ]; then\n\
               exit 0\n\
             fi\n",
            herdr_calls = herdr_calls.display(),
            agent_listed = agent_listed.display(),
            agent_json = agent_json,
            done_agent_json = done_agent_json.unwrap_or_else(|| agent_json.clone()),
        ),
    )
    .unwrap();

    let fake_gh = fake_bin.join("gh");
    std::fs::write(
        &fake_gh,
        format!(
            "#!/bin/sh\nprintf '%s\n' \"$*\" >> '{}'\nprintf '%s\n' '{{\"state\":\"MERGED\",\"mergedAt\":\"2026-08-19T19:00:00Z\",\"headRefOid\":\"merged-head\"}}'\n",
            gh_calls.display(),
        ),
    )
    .unwrap();
    let fake_git = fake_bin.join("git");
    std::fs::write(
        &fake_git,
        "#!/bin/sh\nif [ \"$1 $2 $3\" = \"symbolic-ref --short refs/remotes/origin/HEAD\" ]; then printf 'origin/main\\n'; else exit 2; fi\n",
    )
    .unwrap();
    std::fs::write(&gh_calls, "").unwrap();
    for fake_program in [&fake_br, &fake_herdr, &fake_gh, &fake_git] {
        make_executable(fake_program);
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["drain", workspace.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();
    (
        output,
        std::fs::read_to_string(herdr_calls).unwrap(),
        std::fs::read_to_string(gh_calls).unwrap(),
    )
}

fn assert_in_progress_merged_row(agent_status: Option<&str>, row: &str) {
    let bead_id = format!("it-merged-{row}");
    let (output, herdr_calls, gh_calls) =
        run_in_progress_merged_pr_sweep(row, &bead_id, agent_status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains(&format!("merged: 1 [{bead_id}")),
        "in_progress + {row} + MERGED PR did not honor Merged precedence: {stdout}"
    );
    assert!(
        !stdout.contains(&format!("stalled: 1 [{bead_id}")),
        "Merged was incorrectly parked as Stalled: {stdout}"
    );
    assert_eq!(
        gh_calls,
        format!("pr view lane/{bead_id} --json state,mergedAt,headRefOid\n"),
        "Merged must be probed once and then absorbed across the forced second sweep"
    );
    let removals: Vec<_> = herdr_calls
        .lines()
        .filter(|call| call.starts_with("worktree remove"))
        .collect();
    if agent_status.is_some() {
        assert_eq!(
            removals,
            [format!("worktree remove --workspace workspace-{bead_id}")],
            "the recorded Merged workspace must be reaped:\n{herdr_calls}"
        );
    } else {
        assert!(
            removals.is_empty(),
            "an absent agent has no recorded workspace to reap:\n{herdr_calls}"
        );
    }
}

#[test]
fn restart_sweep_reports_absent_in_progress_merged_pr_as_merged() {
    assert_in_progress_merged_row(None, "absent");
}

#[test]
fn restart_sweep_reports_done_in_progress_merged_pr_as_merged() {
    assert_in_progress_merged_row(Some("done"), "done");
}

#[test]
fn restart_sweep_reports_working_in_progress_merged_pr_as_merged() {
    assert_in_progress_merged_row(Some("working"), "working");
}

#[test]
fn a_dirty_blocked_lane_is_left_standing_and_reported() {
    let workspace = TempDir::new("drain-dirty-blocked");
    let fake_bin = workspace.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let blocked = workspace.0.join("blocked");
    let herdr_calls = workspace.0.join("herdr-calls");

    let fake_br = fake_bin.join("br");
    std::fs::write(
        &fake_br,
        format!(
            "#!/bin/sh\n\
             if [ \"$1 $2\" = \"list --json\" ]; then\n\
               printf '{{\"issues\":[]}}\n'\n\
             elif [ \"$1\" = \"ready\" ]; then\n\
               if [ -f '{blocked}' ]; then printf '[]\\n'; else printf '[{{\"id\":\"it-blocked\",\"title\":\"blocked bead\",\"priority\":0,\"labels\":[]}}]\\n'; fi\n\
             elif [ \"$1 $2 $3\" = \"update it-blocked --claim\" ]; then\n\
               exit 0\n\
             elif [ \"$1 $2\" = \"show it-blocked\" ]; then\n\
               printf '[{{\"status\":\"in_progress\",\"comments\":[{{\"id\":7,\"text\":\"BLOCKED: dirty fixture\"}}]}}]\\n'\n\
             else\n\
               printf 'unexpected br call: %s\\n' \"$*\" >&2; exit 2\n\
             fi\n",
            blocked = blocked.display(),
        ),
    )
    .unwrap();

    let dirty_error = r#"{"error":{"code":"dirty_worktree_requires_force","message":"worktree contains modified or untracked files"}}"#;
    let fake_herdr = fake_bin.join("herdr");
    std::fs::write(
        &fake_herdr,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{herdr_calls}'\n\
             if [ \"$1 $2\" = \"worktree create\" ]; then\n\
               printf '%s\\n' '{{\"result\":{{\"type\":\"worktree_created\",\"workspace\":{{\"workspace_id\":\"blocked-workspace\"}},\"root_pane\":{{\"pane_id\":\"blocked-pane\"}},\"worktree\":{{\"path\":\"{root}\",\"branch\":\"lane/it-blocked\"}}}}}}'\n\
             elif [ \"$1 $2\" = \"agent prompt\" ]; then\n\
               : > '{blocked}'\n\
               printf 'worker settled\\n'\n\
             elif [ \"$1 $2\" = \"worktree remove\" ]; then\n\
               printf '%s\\n' '{dirty_error}' >&2\n\
               exit 1\n\
             fi\n",
            herdr_calls = herdr_calls.display(),
            root = workspace.0.display(),
            blocked = blocked.display(),
            dirty_error = dirty_error,
        ),
    )
    .unwrap();

    let fake_git = fake_bin.join("git");
    std::fs::write(
        &fake_git,
        "#!/bin/sh\nif [ \"$1 $2 $3\" = \"symbolic-ref --short refs/remotes/origin/HEAD\" ]; then printf 'origin/main\\n'; else exit 2; fi\n",
    )
    .unwrap();
    for fake_program in [&fake_br, &fake_herdr, &fake_git] {
        make_executable(fake_program);
    }

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["drain", workspace.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("blocked: 1 [it-blocked"),
        "stdout: {stdout}"
    );
    let calls = std::fs::read_to_string(herdr_calls).unwrap();
    let removals: Vec<_> = calls
        .lines()
        .filter(|call| call.starts_with("worktree remove"))
        .collect();
    assert_eq!(
        removals,
        ["worktree remove --workspace blocked-workspace"],
        "dirty blocked lanes get one non-forced attempt:\n{calls}"
    );
}

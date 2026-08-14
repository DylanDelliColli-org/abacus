//! Integration: real `br` binary against a throwaway workspace, and the real
//! `abacus` binary against real backlogs. Requires `br` on PATH (it is the
//! pinned substrate — a machine that can't run these can't run abacus).

use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::{PermissionsExt, symlink};

use abacus::{BeadOutcome, dispatch_prompt, parse_bead_outcome, parse_ready, select_bead};

struct TempWorkspace(PathBuf);

impl TempWorkspace {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("abacus-it-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempWorkspace(dir)
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn br(dir: &PathBuf, args: &[&str]) -> String {
    let out = Command::new("br")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("br must be on PATH");
    assert!(
        out.status.success(),
        "br {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
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

fn find_on_path(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("test PATH must be set"))
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{program} must be on PATH"))
}

#[test]
fn ready_roundtrip_selects_highest_priority_bead() {
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

#[test]
fn abacus_run_on_empty_backlog_dispatches_nothing_and_exits_zero() {
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
fn abacus_run_claims_the_selected_bead_before_opening_its_lane() {
    let ws = TempWorkspace::new("claim-before-lane");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=claim before dispatch"]);

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let restricted_bin = ws.0.join("restricted-bin");
    std::fs::create_dir(&restricted_bin).unwrap();
    symlink(find_on_path("br"), restricted_bin.join("br")).unwrap();

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

#[test]
fn abacus_run_without_a_tracker_fails_with_brs_own_message() {
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
fn abacus_run_rejects_a_settled_lane_whose_bead_is_still_open() {
    let ws = TempWorkspace::new("open-outcome");
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

    let path = std::env::join_paths(std::iter::once(fake_bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("test PATH must be set"),
    )))
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_abacus"))
        .args(["run", ws.0.to_str().unwrap()])
        .env("PATH", path)
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("never engaged"), "stderr: {stderr}");
}

#[cfg(unix)]
#[test]
fn abacus_run_reaps_a_clean_lane_without_force_after_the_worker_closes_its_bead() {
    let ws = TempWorkspace::new("closed-outcome");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=worker completes"]);

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let fake_herdr = fake_bin.join("herdr");
    let calls = ws.0.join("herdr-calls");
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
    let removal_calls: Vec<_> = calls
        .lines()
        .filter(|call| call.starts_with("worktree remove"))
        .collect();
    assert_eq!(
        removal_calls,
        ["worktree remove --workspace completed-workspace"],
        "Herdr calls:\n{calls}"
    );
}

#[cfg(unix)]
#[test]
fn abacus_run_warns_and_forces_removal_when_a_completed_lane_is_dirty() {
    let ws = TempWorkspace::new("dirty-completed-outcome");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=dirty worker completes"]);

    let json = br(&ws.0, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead = select_bead(&beads).unwrap();

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
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
    let ws = TempWorkspace::new("prompt-retry");
    br(&ws.0, &["init", "--prefix", "it"]);
    br(&ws.0, &["create", "--title=worker survives prompt race"]);

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

#[test]
fn abacus_without_a_command_prints_usage() {
    let out = Command::new(env!("CARGO_BIN_EXE_abacus")).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage"));
}

#[test]
fn dispatch_protocol_closes_pushes_then_opens_pr_and_leaves_lane_clean() {
    let ws = TempWorkspace::new("closed-push");
    let origin = ws.0.join("origin.git");
    let lane = ws.0.join("lane");
    let origin_arg = origin.to_str().unwrap();
    let lane_arg = lane.to_str().unwrap();

    git(&ws.0, &["init", "--bare", origin_arg]);
    git(&ws.0, &["init", "-b", "lane/it-work", lane_arg]);
    br(&lane, &["init", "--prefix", "it"]);
    br(&lane, &["create", "--title=protocol regression"]);

    let json = br(&lane, &["ready", "--json"]);
    let beads = parse_ready(&json).unwrap();
    let bead_id = &beads[0].id;
    let prompt = dispatch_prompt(bead_id, "lane/it-work");
    let close = prompt.find(&format!("br close {bead_id}")).unwrap();
    let stage = prompt.find("git add .beads").unwrap();
    let commit = prompt.find("commit all work").unwrap();
    let push = prompt.find("git push -u origin lane/it-work").unwrap();
    let pr = prompt.find("gh pr create --base main").unwrap();
    assert!(
        close < stage && stage < commit && commit < push && push < pr,
        "dispatch protocol is out of order: {prompt}"
    );
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

    git(&lane, &["add", ".beads"]);
    git(
        &lane,
        &[
            "-c",
            "user.name=Abacus Integration Test",
            "-c",
            "user.email=abacus@example.invalid",
            "commit",
            "-m",
            "seed open bead",
        ],
    );
    git(&lane, &["remote", "add", "origin", origin_arg]);

    br(&lane, &["update", bead_id, "--claim"]);
    br(&lane, &["close", bead_id]);
    git(&lane, &["add", ".beads"]);
    git(
        &lane,
        &[
            "-c",
            "user.name=Abacus Integration Test",
            "-c",
            "user.email=abacus@example.invalid",
            "commit",
            "-m",
            "close completed bead",
        ],
    );
    git(&lane, &["push", "-u", "origin", "lane/it-work"]);

    assert!(git(&lane, &["status", "--porcelain"]).is_empty());
    let remote_tracker = git(
        &lane,
        &[
            "--git-dir",
            origin_arg,
            "show",
            "lane/it-work:.beads/issues.jsonl",
        ],
    );
    let pushed_bead = remote_tracker
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|bead| bead["id"] == bead_id.as_str())
        .expect("pushed branch must contain the bead");
    assert_eq!(pushed_bead["status"], "closed");
}

//! Integration: real `br` binary against a throwaway workspace, and the real
//! `abacus` binary against real backlogs. Requires `br` on PATH (it is the
//! pinned substrate — a machine that can't run these can't run abacus).

use std::path::PathBuf;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use abacus::{parse_ready, select_bead};

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

    let fake_bin = ws.0.join("fake-bin");
    std::fs::create_dir(&fake_bin).unwrap();
    let fake_herdr = fake_bin.join("herdr");
    let lane_json = serde_json::json!({
        "result": {
            "type": "worktree_created",
            "workspace": { "workspace_id": "fake-workspace" },
            "root_pane": { "pane_id": "fake-pane" },
            "worktree": {
                "path": ws.0,
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

#[test]
fn abacus_without_a_command_prints_usage() {
    let out = Command::new(env!("CARGO_BIN_EXE_abacus")).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("usage"));
}

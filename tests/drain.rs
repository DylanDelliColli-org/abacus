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
             if [ \"$1\" = \"ready\" ]; then\n\
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

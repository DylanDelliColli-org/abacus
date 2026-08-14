//! Integration coverage for the `br` PATH shim against real Git worktrees
//! and real scratch `br` stores.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

struct TempFixture {
    root: PathBuf,
}

impl TempFixture {
    fn new(tag: &str) -> Self {
        let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "abacus-shim-{tag}-{}-{sequence}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for TempFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn run(dir: &Path, program: &str, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|error| panic!("failed to run {program} {args:?}: {error}"))
}

fn run_ok(dir: &Path, program: &str, args: &[&str]) -> Output {
    let output = run(dir, program, args);
    assert!(
        output.status.success(),
        "{program} {args:?} failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn init_plain_checkout(fixture: &TempFixture) -> PathBuf {
    let checkout = fixture.path("main");
    std::fs::create_dir(&checkout).unwrap();
    run_ok(&checkout, "git", &["init", "--quiet"]);
    run_ok(
        &checkout,
        "git",
        &["config", "user.email", "shim@test.invalid"],
    );
    run_ok(&checkout, "git", &["config", "user.name", "Shim Test"]);
    run_ok(
        &checkout,
        "git",
        &["commit", "--quiet", "--allow-empty", "-m", "fixture"],
    );
    run_ok(&checkout, "br", &["init", "--prefix", "it"]);
    checkout
}

fn shim_where(dir: &Path) -> PathBuf {
    let shim = Path::new(env!("CARGO_MANIFEST_DIR")).join("bin/br-shim");
    let output = Command::new(&shim)
        .arg("where")
        .current_dir(dir)
        .env_remove("BEADS_DIR")
        .output()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", shim.display()));
    assert!(
        output.status.success(),
        "{} where failed:\nstdout: {}\nstderr: {}",
        shim.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    PathBuf::from(stdout.lines().next().expect("br where must report a store"))
}

#[cfg(unix)]
#[test]
fn linked_worktree_uses_the_main_checkouts_store() {
    let fixture = TempFixture::new("linked");
    let main = init_plain_checkout(&fixture);
    let lane = fixture.path("lane");
    run_ok(
        &main,
        "git",
        &[
            "worktree",
            "add",
            "--quiet",
            "-b",
            "fixture-lane",
            lane.to_str().unwrap(),
        ],
    );

    assert_eq!(shim_where(&lane), main.join(".beads"));
}

#[cfg(unix)]
#[test]
fn plain_checkout_passes_through_to_its_local_store() {
    let fixture = TempFixture::new("plain");
    let checkout = init_plain_checkout(&fixture);

    assert_eq!(shim_where(&checkout), checkout.join(".beads"));
}

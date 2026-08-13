//! `abacus run` — single-pass dispatch: read the ready backlog, open a lane,
//! start a codex worker in it, send the dispatch prompt, wait for settle.
//! Everything stateful is shelled out to `br` and `herdr`; records,
//! acceptance, and evidence chains are deliberately absent (SHIFT-REPORT
//! 2026-08-13 §3).

use std::path::{Path, PathBuf};
use std::process::{Command, exit};

use abacus::{dispatch_prompt, parse_ready, parse_worktree_created, select_bead};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("run") => {
            let repo = args.get(1).map(PathBuf::from).unwrap_or_else(|| ".".into());
            if let Err(e) = cmd_run(&repo) {
                eprintln!("abacus run: {e}");
                exit(1);
            }
        }
        _ => {
            eprintln!("usage: abacus run [repo-path]");
            exit(2);
        }
    }
}

fn cmd_run(repo: &Path) -> Result<(), String> {
    let repo = repo
        .canonicalize()
        .map_err(|e| format!("cannot resolve repo path {}: {e}", repo.display()))?;
    let repo_str = repo.to_string_lossy().into_owned();

    let ready = capture("br", &["ready", "--json"], Some(&repo))?;
    let beads = parse_ready(&ready)?;
    let Some(bead) = select_bead(&beads) else {
        println!("no ready beads in {repo_str}; nothing to dispatch");
        return Ok(());
    };
    println!("selected {} — {}", bead.id, bead.title);

    let branch = format!("lane/{}", bead.id);
    let created = capture(
        "herdr",
        &[
            "worktree",
            "create",
            "--cwd",
            &repo_str,
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
            &bead.id,
            "--kind",
            "codex",
            "--pane",
            &lane.pane_id,
        ],
        None,
    )?;
    println!("codex worker started as agent {}", bead.id);

    let prompt = dispatch_prompt(&bead.id, &lane.branch);
    println!(
        "dispatched; waiting for the lane to settle (Ctrl-C detaches, the lane keeps running)"
    );
    let settled = capture(
        "herdr",
        &["agent", "prompt", &bead.id, &prompt, "--wait"],
        None,
    )?;
    println!("{}", settled.trim_end());
    Ok(())
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

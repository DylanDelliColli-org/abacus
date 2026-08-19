```doc-meta
role: contract
lifecycle: active
```

# Agents

You are in the **abacus** repository: an execution engine over `br` (work
state) and Herdr (agent orchestration). One page; follow the links, they are
the authority.

## Orientation

- For planning work, follow **`.claude/skills/abacus-plan/SKILL.md`**.
- **`NORTH-STAR.md`** is the standard every proposal is judged against. Read
  it before proposing anything.
- **`CONSTRAINTS.md`** carries the four measured findings. They are paid-for
  evidence; do not relearn them the expensive way.
- **Work state lives in `br`.** There is no other tracker and no TODO lists.
  This store mints **`ab-*`** ids. The parts bin `../abacus-v1` has its own
  `br` store minting `abacus-*`, and the two namespaces must stay disjoint —
  a bare id has to name exactly one bead. The nine `abacus-*` ids already in
  this store predate the split; they were checked against all 103 in v1 and
  collide with none, so they stay as they are. Never point this repo's `br`
  at v1's store, or the reverse.

## Working a bead

```sh
br ready                      # what is available
br show <id>                  # your full scope — the description is the spec
br update <id> --claim        # claim before you start
br close <id>                 # only after the work is verified
```

Never use `br edit` (opens an editor and hangs you). Update fields with
`br update <id> --description "..."` / `--notes "..."`.

## Tracker merge driver

`.gitattributes` assigns `.beads/issues.jsonl` to the `beads-jsonl` merge
driver. Git merge-driver configuration is local to each clone, so configure
it after cloning (with `abacus` on `PATH`):

```sh
git config merge.beads-jsonl.driver 'abacus merge-jsonl %A %O %B'
```

The driver unions issue IDs from all three snapshots and keeps the complete
line with the latest `updated_at`. If any line cannot be parsed, the driver
exits non-zero so Git leaves a normal conflict for manual resolution.

## Shared work-state wrapper

`bin/br-shim` binds `br` commands run in linked worktrees to the main
checkout's `.beads` store. It is **installed** on this machine
(2026-08-14): `~/.local/shims/br` is a symlink to the repository's
`bin/br-shim`, and `~/.zshrc` prepends `~/.local/shims` to `PATH`. Repo
updates to the shim propagate through the symlink; if the repository
directory ever moves, the symlink fails loudly (`br` stops resolving) —
re-point it rather than debugging br.

On a machine without the wrapper, bind every worker command inline:

```sh
BEADS_DIR=<main-checkout>/.beads br <arguments>
```

Do not rely on an exported `BEADS_DIR`: worker shell invocations do not share
environment changes.

Four `br doctor` WARNs on this store are known-benign — do not jot them, only
new findings: `br_path_dupes` (the shim above is deliberate),
`db.recovery_artifacts` (doctor's own aged check governs escalation),
`base_jsonl` stale merge anchor (the next `br sync --flush-only` refreshes
it), and `dep.dead_closed_blocking_edges` (satisfied-history edges; nothing
is blocked).

## Lanes

- A worker lane is a git worktree under `~/.herdr/worktrees/abacus/` on a
  branch named `lane/<bead-id>`. Do all work inside your own lane.
- Write the failing test first; implement until green. Pin verification to the
  workspace MSRV (`Cargo.toml` `rust-version`, currently 1.85): if needed,
  install it once with `rustup toolchain install 1.85 --profile minimal
  --component clippy --component rustfmt`, then run the full suite
  (`RUSTUP_TOOLCHAIN=1.85 cargo test`), plus `RUSTUP_TOOLCHAIN=1.85 cargo
  clippy` and `RUSTUP_TOOLCHAIN=1.85 cargo fmt --check`.
- Commit and self-push your lane branch: `git push -u origin lane/<bead-id>`.
- **Worker autonomy ends at the PR.** Workers never merge to `main` or any
  other default branch. In land mode, the engine — never a worker — enqueues
  validated PRs into the repository's merge queue, which performs the merge
  ([ADR 0003](docs/adr/0003-pr-validation-and-auto-merge.md)). Without land
  mode, the operator reviews at the merge boundary.
- **Pass Markdown to `gh` by file, never inline.** Use `gh pr comment <n>
  --body-file <path>` (and `--body-file` wherever a body is accepted): a
  double-quoted `--body` string goes through the shell first, and backtick
  code spans execute as command substitution before `gh` runs (observed
  2026-08-19: a rework-brief body executed embedded command text).
- **Read-only review dispatches** (bloat review, spec validation) must state
  in the prompt that the review is not bead-tracked work — no beads, no
  branches, no commits. A reviewer that follows the prime directive without
  this line leaves tracker and remote exhaust.

## If you are lost

Your bead id is in your branch name (`git branch --show-current`) and your
dispatch prompt. `br show <that id>` restores your scope. If you cannot
proceed, say BLOCKED and why, and stop — do not improvise scope.

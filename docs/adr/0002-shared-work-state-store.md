```doc-meta
role: contract
lifecycle: active
```

# ADR 0002: One shared work-state store for all lanes

- **Status:** draft — bloat review complete (fresh Codex context, three
  cuts: operator applied the zyb-ordering sentence as a shrink and the
  cross-machine trim, and reaffirmed committed completion records on the
  crash-first-class constraint); spec validation complete (second fresh
  Codex context, three findings, all applied by operator decision:
  close-last worker protocol so a closed bead implies a reviewable PR,
  per-command BEADS_DIR binding because worker shell invocations do not
  share exports, and the concurrency evidence narrowed to its measured
  read-side); carriage mechanism debate validated by a third fresh Codex
  context with empirical fixtures (symlink swap rejected — the lane is
  git-dirty by construction; the PATH wrapper validated with all engine
  call sites confirmed interceptable; upstream discovery confirmed
  nonexistent in br 0.1.45), operator selected the wrapper with the
  per-command binding as fallback; pending final operator approval
- **Date:** 2026-08-14
- **Deciders:** operator (direction), orchestrator session (record)
- **Authority:** NORTH-STAR.md thesis ("teams of provider-agnostic agents
  autonomously clear it") and its success condition's merge-conflict clause;
  CONSTRAINTS.md finding 1 (the `br` concurrency evidence) and finding 3
  (the launch environment carries identity).

## Context

Two root causes made tracker conflicts a permanent feature of sitting PRs.
First, every worker lane carries its own copy of `.beads` — an accident of
git worktrees copying the tree — and the dispatch protocol had each lane
commit its bead close to its branch, so every PR edits
`.beads/issues.jsonl` while main's tracker moves. Second, GitHub computes
PR mergeability with its own merge machinery and never runs a repository's
custom merge driver, so the `abacus merge-jsonl` driver (ADR 0001 era,
bead ab-s1x) fixes local merges but leaves PRs showing conflicts that only
a local pre-merge-and-push clears — a manual step the orchestrator absorbed
on every PR.

The two-lane stress test (bead ab-h3v, 2026-08-14) sharpened the picture:
with the dispatch-time claim landing in the main store, the orchestrator's
reconcile-closes — rewriting the very lines the PRs carried — turned out to
be the main manufacturer of same-line conflicts, and retiring that practice
let two concurrent worker PRs sit mergeable. The residual class remains:
at overnight scale, parallel PRs each carrying tracker edits against a
moving main will collide again.

## Decision

Every worker lane uses the **main checkout's `br` store** directly. Lane
branches never touch `.beads`.

- **Carriage:** a **PATH-precedent `br` wrapper** installed once on the
  operator's machine. When invoked from a linked worktree it resolves the
  main checkout through git's common-dir metadata and executes the real
  `br` with `BEADS_DIR` set; anywhere else it passes through untouched.
  Plain `br` commands then bind to the shared store for workers, the
  engine, and the operator alike — every current call site resolves
  through PATH, verified live. Two recorded boundaries: a call that
  bypasses PATH (absolute path to the real binary) rediscovers the lane
  store, and a machine without the wrapper falls back to the per-command
  inline binding `BEADS_DIR=<main-checkout>/.beads br …` (session exports
  do not survive a worker's independent shell invocations). Worktree-aware
  discovery native to `br` is the eventual upstream fix; until it ships,
  the wrapper is the mechanism.
- **Worker protocol:** claim and close run against the shared store. The
  `git add .beads` step is removed; a lane commits only source and test
  changes. The close is the worker's **last act, after the push succeeds
  and the PR exists** — so a `closed` bead means the work is reviewable,
  and a failed push or PR leaves the bead `in_progress` for the probe to
  report honestly.
- **Outcome probe:** `abacus run` reads the shared store from the main
  checkout after settle. The evidence rule is unchanged — bead state, not
  runtime signals.
- **Completion record:** the shared store is the single record of claim
  and close, committed to git by the orchestrator's ordinary backlog
  commits on main.

This deliberately reverses the close-before-push protocol decision (bead
ab-zyb): the branch-carried completion record proved redundant in practice
— the orchestrator reconciled every close into main anyway — and was the
structural source of the conflict class. With a live shared store, the
ordering zyb regulated stops mattering for tracker carriage and PR
mergeability; for outcome evidence it inverts — the close moves to the
end of the worker sequence, after the push and the PR, so that bead
state remains a truthful completion signal.

## Consequences

- PRs carry only source changes; the tracker conflict class disappears
  structurally rather than being resolved faster.
- Claims and closes are visible live across all lanes and the orchestrator
  — the claim-visibility gap closes as a side effect.
- Reaps of completed lanes are clean by construction, so the
  clean-removal-first tripwire (bead ab-irn) signals only genuine source
  dirt.
- The `abacus merge-jsonl` driver remains for main-line merges across
  sessions; it stops being needed per PR.
- The shared-store concurrency evidence is read-side: finding 1's
  measurements — 879 reads, zero timeouts, p50 51ms — were taken on one
  shared `br` store under 11 concurrent claimants. Concurrent write
  behavior is unmeasured; the no-additional-locking decision stands on
  `br`'s own locking, and the first concurrent-lane run after this lands
  is its live measurement.
- A lane crash leaves no tracker debris in the lane; the shared store and
  its committed JSONL snapshots are the recovery surface, per the
  crash-first-class constraint.

## Not built now

No `br` daemon or server. No additional locking layer — `br`'s own
locking is the mechanism the evidence already validated. No engine-side
mirroring of worker writes. No change to the reap or PR protocol beyond
removing the tracker-commit step. Each waits for an observed failure.

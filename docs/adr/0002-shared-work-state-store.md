```doc-meta
role: contract
lifecycle: active
```

# ADR 0002: One shared work-state store for all lanes

- **Status:** draft — pending bloat review, spec validation, and operator
  approval
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

- **Carriage:** the dispatch prompt instructs the worker to export
  `BEADS_DIR` pointing at the main checkout's `.beads` before any `br`
  command — prompt carriage, the same mechanism that already carries the
  bead identity (CONSTRAINTS.md finding 3).
- **Worker protocol:** claim and close run against the shared store. The
  `git add .beads` step is removed; a lane commits only source and test
  changes.
- **Outcome probe:** `abacus run` reads the shared store from the main
  checkout after settle. The evidence rule is unchanged — bead state, not
  runtime signals.
- **Completion record:** the shared store is the single record of claim
  and close, committed to git by the orchestrator's ordinary backlog
  commits on main.

This deliberately reverses the close-before-push protocol decision (bead
ab-zyb): the branch-carried completion record proved redundant in practice
— the orchestrator reconciled every close into main anyway — and was the
structural source of the conflict class. What zyb actually established
survives: the close still happens before the push in the worker's
sequence; only its location changes from the lane's copy to the shared
store.

## Consequences

- PRs carry only source changes; the tracker conflict class disappears
  structurally rather than being resolved faster.
- Claims and closes are visible live across all lanes and the orchestrator
  — the claim-visibility gap closes as a side effect.
- Reaps of completed lanes are clean by construction, so the
  clean-removal-first tripwire (bead ab-irn) signals only genuine source
  dirt.
- The `abacus merge-jsonl` driver remains for main-line merges across
  sessions and machines; it stops being needed per PR.
- Concurrent writes to one store are the proven path, not new risk:
  finding 1's measurements — 879 reads, zero timeouts, p50 51ms — were
  taken on one shared `br` store under 11 concurrent claimants.
- A lane crash leaves no tracker debris in the lane; the shared store and
  its committed JSONL snapshots are the recovery surface, per the
  crash-first-class constraint.

## Not built now

No `br` daemon or server. No additional locking layer — `br`'s own
locking is the mechanism the evidence already validated. No engine-side
mirroring of worker writes. No change to the reap or PR protocol beyond
removing the tracker-commit step. Each waits for an observed failure.

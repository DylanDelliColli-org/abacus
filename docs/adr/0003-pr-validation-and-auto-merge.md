```doc-meta
role: contract
lifecycle: active
```

# ADR 0003: PR validation and auto-merge via the GitHub merge queue

- **Status:** **draft** 2026-08-15, pending operator acceptance at the
  `ab-automerge-2b2` RECORD gate. Trail: bloat review run 1 (fresh
  Codex context, pre-amendment) superseded by run 2 (fresh Codex pane,
  post-amendment); run 2's findings — cut 1 withdrawn by the reviewer
  after operator clarification and escalated into the merge-queue
  pivot this ADR now records, cuts 3 and 6 accepted and absorbed by
  the pivot, cuts 2, 4, 5 reaffirmed by operator decision. Spec
  validation pending.
- **Date:** 2026-08-15
- **Deciders:** operator (direction; recorded decisions across the
  FRAMING, RESEARCH, and TEST-STRATEGY gates, the north-star
  amendment, and the merge-queue pivot), orchestrator session (record)
- **Authority:** NORTH-STAR.md success condition as amended 2026-08-15
  ("On repositories the operator opts in, the engine also merges
  pending PRs during the overnight run — serially, each validated
  against the main it actually lands on"); CONSTRAINTS.md findings
  2–4; ADR 0002's close-last worker protocol, which this ADR relies on
  as its admission predicate.

## Context

The execution loop demonstrated on 2026-08-13/14 ends at the PR:
workers push, open PRs, and close their beads; the operator merges by
hand each morning. An overnight multi-bead run leaves main moving only
when merges happen — without engine-side merging, PRs pile up against
one base and conflict serially during manual merges, or the operator
intervenes overnight, which the success condition forbids. The
operator directed an autonomous mode for lower-risk repositories where
merge throughput matters more than morning review, directed that CI
become standard across their repositories, and amended the north star
accordingly (2026-08-15).

Planning ran as a full-tier `abacus-plan` epic (`ab-automerge-2b2`);
this ADR compresses its gated FRAMING (stories S1–S9), RESEARCH,
ARCHITECTURE with its merge-queue addendum, and TEST-STRATEGY
sections; the planning record's git history holds the full trail.

Two facts shaped the final design. First, the close-last protocol is
test-enforced, so a closed bead implies a pushed branch and an
existing PR — the engine can rediscover any PR from its
`lane/<bead-id>` branch name with no new persisted state. Second, an
engine-owned merge loop was designed in full and then substantially
deleted at review: the initial architecture rejected GitHub's native
merge queue on the grounds that it validates a speculative merge
commit. The bloat reviewer, after the operator supplied a new fact (a
GitHub organization exists and the repository is public, making the
queue available after a repo transfer), correctly reversed that
reasoning — the `merge_group` commit GitHub constructs from the latest
base plus preceding queued PRs is *exactly* the moving-base candidate
that remote validation must cover, and a merge limit of one makes FIFO
landing strictly serial. The operator adopted the pivot.

## Decision

**GitHub's merge queue owns ordering, candidate construction, remote
validation, and the merge itself.** The repository is transferred to
the operator's organization and configured with branch protection,
the portable CI jobs as required status checks, and a merge-queue
ruleset with merge limit 1 — an operator-act prerequisite bead that
all implementation work depends on.

**Abacus does three things around the queue**, via two subcommands
beside the existing `abacus run`:

- **`abacus drain [repo]`** — the multi-bead dispatch loop: while
  label-eligible ready beads exist, run one dispatch cycle to settle
  and reap, then reselect. A failed or already-taken claim is a normal
  event: reselect, never abort. Lane concurrency comes from running
  multiple drain processes.
- **`abacus land [repo]`** — **admission → enqueue → exception
  watch**, serialized, with `--once` as the finite test-entry mode.
  Running `land` on a repository is the auto-merge opt-in; not running
  it leaves the morning-review default untouched. At startup, land
  refuses a repository whose merge queue or required checks are not
  configured.
  - **Admission:** full local validation — suite, clippy, fmt,
    including the `br`-dependent integration tests — of the PR branch
    composed with current origin/main in a throwaway, unpushed
    worktree. A red admission parks the PR with evidence; a
    composition that conflicts routes to the exception handler without
    enqueueing.
  - **Enqueue:** add the PR to the merge queue (the gh verb is
    verified at implementation). GitHub then validates the
    `merge_group` with the portable CI (whose workflow carries the
    `merge_group:` trigger) and merges FIFO.
  - **Exception watch:** a PR the queue dequeues, or one that
    conflicts at admission, gets **exactly one** agent-resolution
    attempt in a fresh herdr lane on the PR branch — the launch
    carries bead id, attempt marker, and explicit resolution framing
    (CONSTRAINTS findings 2–3) — then re-enqueues on green
    re-admission or **parks**: the PR stays open with a
    `gh pr comment` carrying the failure evidence, the bead stays
    closed, the tracker is never written, and the run continues.

**Validation legs are asymmetric by decision:** the local admission
leg is the full-parity gate; the queue's CI validates the portable
subset (28 tests plus clippy and fmt) on the exact landing
composition. The `br`-dependent tests therefore never run on the
`merge_group` itself — a recorded tradeoff, deferred until an observed
failure; the revival path is installing `br` in CI.

**Crash recovery is stateless recomputation** (CONSTRAINTS finding 4):
the queue itself lives on GitHub; candidates re-derive from GitHub and
the `br` store on every start; admission worktrees are throwaway;
nothing about queue position is persisted on the host.

**Code shape:** `src/land.rs` is a pure policy module — eligibility
parsing, enqueue-result parsing, queue-state reading, park-evidence
construction — fixture-tested; process-spawning gains one
exit-code-aware sibling of `capture()`, with `capture()` and its call
sites untouched. `BeadOutcome` is not extended. Forbidden always:
`gh pr merge --admin`, force-push, rebase of lane branches,
`gh pr update-branch`, and `-X` merge strategy options in any engine
git invocation.

**Generality (S9):** the worker prompt reads the repository's default
branch instead of hardcoding `--base main`, preserving verbatim the
`push < pr < close` prompt assertion the admission predicate rests on;
the `br` shim resolves the real binary through a `BR_REAL` override
with the current path as fallback.

**CI groundwork (S8):** a standard workflow — test, clippy
`-D warnings`, fmt `--check` — on `pull_request`, `push` to the
default branch, and `merge_group`, ships first on this repository,
with `Cargo.toml`'s `rust-version` as the single toolchain pin the
workflow reads. Acceptance is verify-by-first-run: first two runs
green, durations recorded on the bead.

**Worker contract:** unchanged. Workers still never merge; AGENTS.md
gains the engine-side exception for land mode. The default
(no `land` process) behaves exactly as today, and the run path is
regression-tested to never touch `gh`.

## Consequences

- Serial landing against the true moving base is GitHub's guarantee,
  not engine code: each `merge_group` is built from the latest main
  plus the queue ahead of it, validated by required checks, merged
  FIFO with limit 1. The machinery abacus does not build — branch
  updates, readiness polling, compare-and-swap merges, branch
  deletion — is machinery abacus cannot get wrong.
- The overnight success condition becomes reachable end to end:
  drain processes fill and clear lanes while one land process per
  opted-in repository admits and enqueues; by morning, merged main
  plus parked PRs with evidence comments are the report.
- A red PR cannot land: admission blocks it locally, required checks
  block it remotely, and `--admin` — the only bypass — is forbidden
  and tested for.
- Parks are silent overnight by design — evidence lives on the PR;
  morning review reads parked PRs, not logs.
- The engine takes a hard dependency on GitHub org-hosted merge-queue
  availability for opted-in repositories (public org repos, or
  private on Enterprise Cloud). Repositories that cannot meet it
  simply stay in the morning-review default.
- The test contract shrinks with the deleted machinery and gains
  admission/enqueue/exception coverage; the revised matrix and budget
  live in the planning record's TEST-STRATEGY delta.
- The operator's merge act shrinks to reviewing parked PRs and any
  repository not opted in.

## Not built now

Each waits for an observed failure or an explicit operator ask: the
engine-owned merge loop (designed, then deleted at review — branch
update, readiness polling, CAS merge, branch deletion); `br` in CI
(the merge-group parity revival path); `git rerere` and new domain
merge drivers as engine layers; a configuration file (opt-in stays
invocation); per-bead risk tiers; an agent review leg before merge;
more than one resolution attempt; rollback automation; deploy-side
machinery; multi-repo coordination beyond one land process per
repository; third-party distribution.

```doc-meta
role: contract
lifecycle: active
```

# ADR 0003: Engine-owned PR validation and auto-merge

- **Status:** **draft** 2026-08-15, pending operator acceptance at the
  `ab-automerge-2b2` RECORD gate. Trail: [bloat review and spec
  validation pending; filled at the gate]
- **Date:** 2026-08-15
- **Deciders:** operator (direction; twelve recorded decisions across the
  FRAMING, RESEARCH, and TEST-STRATEGY gates), orchestrator session
  (record)
- **Authority:** NORTH-STAR.md thesis, success condition, and non-goals
  as amended 2026-08-15 — the amendment lands with this ADR and cites
  it; CONSTRAINTS.md findings 2–4; ADR 0002's close-last worker
  protocol, which this ADR relies on as its admission predicate.

## Context

The execution loop demonstrated on 2026-08-13/14 ends at the PR: workers
push, open PRs, and close their beads; the operator merges by hand each
morning. Two pressures broke that boundary. First, an overnight
multi-bead run leaves main moving only when merges happen — without
engine-side merging, either nothing lands until morning (PRs pile up
against one base, then conflict serially during manual merges) or the
operator intervenes overnight, which the success condition forbids.
Second, the operator directed an autonomous mode for lower-risk
repositories where merge throughput matters more than morning review —
and separately directed that CI become standard across their
repositories, and that the merge machinery be a general, repo-agnostic
capability rather than an abacus special case.

Planning ran as a full-tier `abacus-plan` epic (`ab-automerge-2b2`),
whose gated FRAMING (stories S1–S9), RESEARCH, ARCHITECTURE (decisions
D1–D15), and TEST-STRATEGY (44-test contract) sections this ADR
compresses; the planning record's git history holds the full trail.

Research established the load-bearing facts: the close-last protocol is
test-enforced, so a closed bead implies a pushed branch and an existing
PR; the engine can rediscover any PR from its `lane/<bead-id>` branch
name with no new persisted state; `gh pr checks` conflates "CI failed"
with "no CI configured" while `gh pr view --json statusCheckRollup`
distinguishes them; `gh pr merge --match-head-commit` provides a
compare-and-swap against post-close pushes; and GitHub's native merge
queue cannot serve the design — it validates a speculative merge commit
the local suite cannot run against, owns ordering the engine needs, and
dequeues conflicts rather than resolving them.

## Decision

Auto-merge is an engine-owned, serialized merge queue, delivered as two
subcommands beside the existing `abacus run`:

- **`abacus drain [repo]`** — the multi-bead dispatch loop: while
  label-eligible ready beads exist, run one dispatch cycle to settle and
  reap, then reselect. A failed or already-taken claim is a normal
  event: reselect, never abort. Lane concurrency comes from running
  multiple drain processes.
- **`abacus land [repo]`** — the merge queue: enumerate candidates
  (open PRs on `lane/*` branches whose bead is closed), process them
  one at a time, poll between rounds; `--once` processes the current
  set and exits. Running `land` on a repository **is** the auto-merge
  opt-in; not running it leaves the morning-review default untouched.
  Land refuses a repository whose PRs report no CI checks — with no CI,
  a broken repository and a clean one are indistinguishable to the
  gate.

**The landing cycle per PR:** update the branch by merging origin's
default branch into it, in a land-owned plain git worktree (lane
worktrees are already reaped by close time) — never rebase, never
force-push, never `gh pr update-branch`; run the local validation leg
(full suite, clippy, fmt — full parity including the `br`-dependent
integration tests); push; wait for the remote leg (CI green on that
exact head SHA, read from `statusCheckRollup` and polled until
`mergeable`/`mergeStateStatus` are known); then merge with
`gh pr merge --merge --match-head-commit <validated SHA>` and delete
the branch. A `BEHIND` result at the pre-merge recheck loops back to
update-and-revalidate. During a land run, land is the sole writer to
the repository's main; `--admin` and `--auto` are forbidden flags.

**Validation legs are asymmetric by decision:** the local leg is the
full-parity gate; CI validates the portable subset (the `br`-dependent
tests remain local-only until `br` has an install recipe). CI presence
is still mandatory for eligibility.

**Conflicts resolve in layers, cheapest first:** domain merge drivers
(the `merge-jsonl` precedent — resolve mechanically where file
semantics permit, fail loudly into a normal conflict otherwise); `git
rerere` as a replay accelerator within a queue drain; then exactly one
agent-resolution attempt in a fresh herdr lane on the conflicted
branch, whose launch carries bead id, attempt marker, and explicit
resolution framing (CONSTRAINTS findings 2 and 3), and whose exit
condition is the local validation leg. Anything unresolved — or any
resolution that validates red — **parks**: the PR stays open with a
`gh pr comment` carrying the failure evidence, the bead stays closed,
the tracker is never written, and the queue moves on.

**Crash recovery is stateless recomputation** (CONSTRAINTS finding 4):
the queue re-derives from GitHub and the `br` store on every start;
worktrees are disposable; resolution commits are pushed as they are
made so an uncommitted worktree never holds the only copy; queue
position is never persisted.

**Code shape:** gate policy — gh JSON parsing, `CiState`, the merge
decision table — lives in a new pure module (`src/land.rs`),
fixture-tested; process-spawning gains one exit-code-aware sibling of
`capture()` (needed because `gh` distinguishes pending from failed by
exit code), with `capture()` and its eight call sites untouched.
`BeadOutcome` is not extended — landing states are not bead-outcome
states.

**Generality (S9), decided at the TEST-STRATEGY gate:** the worker
prompt reads the repository's default branch instead of hardcoding
`--base main`, preserving verbatim the `push < pr < close` prompt
assertion this ADR's admission predicate rests on; the `br` shim
resolves the real binary through a `BR_REAL` override with the current
path as fallback. One land integration fixture runs on a `trunk`
default branch to keep the path honest.

**CI groundwork (S8):** a standard workflow — test, clippy `-D
warnings`, fmt `--check`, on PR and push to main — ships first on this
repository, with `Cargo.toml`'s `rust-version` as the single toolchain
pin the workflow reads. Acceptance is verify-by-first-run: first two
runs green, durations recorded on the bead.

**Worker contract:** unchanged. Workers still never merge; AGENTS.md
gains the engine-side exception in land mode. The morning-review
default (no `land` process) behaves exactly as today, and the run path
is regression-tested to never touch `gh`.

## Consequences

- The moving base becomes the mechanism: each PR revalidates against
  the main it actually lands on, serialized, so a stale-green PR cannot
  land on old evidence. (Honestly noted: the cross-lane conflict rate
  observed to date is ~zero — main's inter-PR movement has been
  tracker-only, and ADR 0002 removed that class. The queue builds ahead
  of measurement because S3 changes the regime by design.)
- The overnight success condition becomes reachable end to end:
  drain(s) fill and clear lanes while one land process per repository
  merges validated work; by morning, merged main plus parked PRs with
  evidence comments are the report.
- A red PR structurally cannot merge: classifier, decision table, and
  process behavior are each separately tested, and every land
  integration test asserts the forbidden-flag invariant in teardown.
- Parks are silent overnight by design — evidence lives on the PR; the
  morning review reads parked PRs, not logs.
- The suite grows by the 44-test contract (~12s projected against the
  30s budget, measured baseline 5.36s).
- The operator's merge act shrinks to reviewing parked PRs and any
  repository not opted in.

## Not built now

Each waits for an observed failure or an explicit operator ask: a
configuration file (opt-in stays invocation); per-bead risk tiers; an
agent review leg before merge; more than one resolution attempt;
rollback automation; deploy-side machinery; `br` in CI; GitHub's native
merge queue; multi-repo coordination beyond one land process per
repository; third-party distribution.

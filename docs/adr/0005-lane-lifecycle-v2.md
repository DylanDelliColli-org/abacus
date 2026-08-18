```doc-meta
role: contract
lifecycle: active
```

# ADR 0005: Lane lifecycle v2 — sweep-based drain, adversarial review gate, warm lanes

- **Status:** **accepted** 2026-08-18, operator approval at the
  `ab-lifecycle-v2-go4` RECORD gate, including ratification of the D6
  amendment (AwaitingReview as run's nominal exit-0 outcome). Governing north star: this
  repository's own `NORTH-STAR.md` (anchor stated explicitly per the
  cross-repo anchor lesson recorded in bb-skills `skills-y13`). Bloat
  trail (fresh Codex pane, one pass, four cuts, operator-disposed
  2026-08-18): cut 1 (defer the whole review cluster as violating
  agent-team acceptance) rejected — the gate is the merge-boundary human
  gate made structural, close-last stays agent-side, adjudication is
  morning review, and the drain never blocks on it; the reviewer's
  revive-when condition (adjudication moved inside the agent team) is
  noted as a possible future evolution in which check-flip authority
  moves to an agent reviewer for overnight runs — a fresh operator
  ruling either way. Cut 2 (drop extraction-first) rejected — it is what
  makes the new states testable. Cut 3 (shrink the test contract)
  accepted as a trim: the ephemeral runtime projection removed, the
  durable counts/caps/flake-retry retained since this ADR outlives the
  planning record. Cut 4 (defer run exit code 3) rejected as a
  reaffirmation of the TEST-STRATEGY gate ruling. Spec validation
  complete (second fresh Codex context, own pane, five findings, all
  applied 2026-08-18): the cross-cycle state blocker resolved by
  deriving cycle bookkeeping from durable facts (adjudicated-head SHA in
  the adjudication grammar, verdict-heading counting, deterministic
  reviewer names); the Completed/AwaitingReview exit-code overlap
  resolved in lane-state's favor (the D6 amendment, ratified at the
  RECORD gate); the per-finding adjudication dispositions restored to
  D4; the herdr and GitHub summaries narrowed to exactly what RESEARCH
  measured.
- **Date:** 2026-08-18
- **Deciders:** operator (all directional rulings, four planning gates,
  the S3 enforcement ruling, the exit-code ruling), orchestrator session
  (record). Producers: sherlock-type RESEARCH and columbo-type
  TEST-STRATEGY subagents; their sections in `PLANNING-ab-lifecycle-v2-go4.md`
  (git history after handoff) carry the full evidence.
- **Authority:** `NORTH-STAR.md` success condition ("zero operator
  interventions" for an overnight multi-bead drain); ADR 0001 (planning
  flow; this ADR is its RECORD artifact); ADR 0002 (shared store,
  close-last protocol — the review gate builds on closed-bead-implies-PR);
  ADR 0003 (GitHub owns merge enforcement — extended here to status-check
  enforcement); operator rulings of 2026-08-17/18 recorded in `ab-phr`,
  `ab-co5`, `ab-blocked-lane-outcome-6bs`.

## Context

Three operator-directed features arrived in one week of production use:
an adversarial PR review gate whose manual flow REFUTED both wave-1 PRs
with real blockers (market-brief PRs 25/26, five findings accepted); warm
worker lanes, after rework cycles paid a measured 8–15 minute fresh-lane
orientation tax against ~5 minutes warm; and drain resilience, after two
contract-compliant BLOCKED lanes each aborted an `abacus run` and required
manual pane forensics. They are one design: what a lane IS between first
dispatch and merge.

Two unknowns were resolved by experiment before this record. First, the
reported herdr monitoring failure is a state-name mismatch, not topology:
codex agents settle at status `done` and never re-enter `idle`, so
`agent wait --until idle` never fires against a settled codex agent
(measured in both split-pane and dedicated topologies; a pre-first-turn
agent still reads genuinely `idle` and matches instantly).
`agent prompt --wait` works in both tested topologies;
`agent wait --until done` was verified in a dedicated root pane only.
All monitoring evidence is codex-kind on the current herdr build. Second,
required-check availability is plan-gated per repo, independently of
auth: on the current production target (private, free plan) the
configuration endpoints return the plan-upgrade refusal even to the
admin account — the plan gate was the blocking gate there. On
plan-eligible repositories, configuring enforcement still requires
account-level admin.

## Decision

**D1 — Outcome classification is two stateless layers.** `BeadOutcome`
(a pure function of `br show` output) gains `Blocked`: bead
`in_progress` AND the highest-id comment's leading token is `BLOCKED`.
A later non-BLOCKED comment supersedes; classification is by comment id,
never array position; a closed bead is `Completed` regardless of
comments. A drain-level `LaneState` is re-derived per cycle from three
probes — `br show`, `herdr agent list`, `gh pr view` on the
deterministic branch `lane/<bead-id>` — into: `Authoring`, `Blocked`,
`AwaitingReview`, `ReworkRequested`, `Merged`, `Stalled`. An accepted
adjudication whose PR has not merged remains `AwaitingReview` with a
flipped status; no separate state. **Cycle bookkeeping is derived from
durable facts only:** the branch head SHA, the count and cycle numbers
of verdict-comment headings, the latest adjudication (which records the
head SHA it adjudicated — D4), and the presence of live lane/reviewer
agents under their deterministic names. `ReworkRequested` holds only
while the branch head still equals the latest rework adjudication's
adjudicated head; new commits mean the rework was performed and the lane
returns to `AwaitingReview` for re-review. Within `AwaitingReview`, the
reviewer-launch action fires only when no verdict comment exists for the
current head and no live reviewer agent exists for the lane; a
posted-but-unadjudicated verdict waits for the operator and never
triggers a relaunch. A crash in the narrow window between reviewer
launch and verdict post may at worst duplicate one review — accepted as
rare and harmless.

**D2 — The drain is sweep-then-dispatch, stateless, serially active.**
Each iteration first re-derives every live lane and acts on transitions
(launch reviewer, flip status, redispatch rework, reap merged, park
blocked/stalled with per-class reporting), then dispatches at most one
new ready bead. The engine persists nothing: after a crash, every lane
reconstructs from deterministic names plus substrate queries. At most one
worker turn is active at any moment (the engine blocks on
`prompt --wait`); concurrency exists only as settled lanes awaiting
adjudication. The drain never aborts on `Blocked`, `AwaitingReview`, or
`Stalled`; it exits when no ready beads remain and no transition is
pending, printing a per-class summary (the morning report).

**D3 — The review gate is engine-owned; reviewers are ephemeral.** When a
lane reaches `AwaitingReview`, the engine generates a refutation brief
from the bead (authority map, per-bead refutation targets, read-only
ground rules with exactly one permitted write — `gh pr comment` on the
target PR — and the required verdict grammar: `REFUTED` / `NOT REFUTED`,
numbered findings, a Probes section), writes it to a gitignored tmp path
in the target repo, and launches a fresh codex context in its own
dedicated herdr workspace under a deterministic per-bead-per-cycle agent
name (the liveness signal D1's bookkeeping reads), prompted by file path
and monitored with `prompt --wait`. Reviewer auth is codex subscription
OAuth by construction. Reviewers never carry context between cycles — the
author-warm / reviewer-ephemeral asymmetry is deliberate design. The
reviewer works from the target repo's main checkout (production-proven; a
same-branch worktree is impossible while the warm lane holds the branch).

**D4 — Two-comment convention; the engine parses adjudications only.**
The reviewer posts its full unadjudicated verdict as a PR comment. After
the operator rules, the adjudication comment is posted human-side **on
the PR — the PR comment stream is the only adjudication parse surface;
bead comments carry rework specs and operator notes and are never parsed
as adjudications.** Its grammar preserves the production convention in
full: the heading
`## Adjudication — cycle <k>`, the overall verdict (accepted or rework),
**per-finding dispositions** — accepted, rejected, or rerouted, each
with its destination (rework-spec expectation, out-of-scope bead id, or
fix commit SHA) — and the **adjudicated head SHA** of the branch state
the ruling covered (the durable anchor D1's cycle bookkeeping and D5's
rework generation both depend on). The engine machine-parses exactly two
textual signals: the worker's `BLOCKED` comment token and adjudication
comments. Reviewer verdict comments are recognized **by heading only**
(`## Adversarial review — cycle <n>`) for existence and cycle counting;
their bodies are never parsed, and an unadjudicated `REFUTED` blocks
nothing. Status
lifecycle via the commit-status API only (check runs are
GitHub-Apps-only), context `adversarial-review`: `pending` posted once at
review launch, flipped to `success` only by an accepting adjudication,
and `failure` is never posted — a refuted PR is being reworked; abandoned
work is a human PR-close. The engine never writes branch-protection or
ruleset configuration: making the check REQUIRED is an onboarding act on
repos whose plan supports it (operator ruling; extends ADR 0003's
GitHub-owns-enforcement posture). A status reader must distinguish
zero-statuses (which the combined endpoint also reports as "pending")
from a posted pending status.

**D5 — Warm rework; reap on merge.** On `ReworkRequested`, the engine
prompts the existing agent (name re-derived via `sanitize_agent_name`)
with a rework spec generated from the adjudication comment, on the same
branch, so the PR updates in place. If the agent has died, the lane is
recreated on the same `lane/<bead-id>` branch (the implementing bead
verifies herdr's worktree-create behavior against an existing branch,
with a git-worktree fallback). Reaping moves from settle to: `Merged`
(force allowed, as today) or operator abandon; `Blocked` lanes reap only
when clean, inverting the existing force path via the dirty-worktree
error discrimination; `AwaitingReview`, `ReworkRequested`, and `Stalled`
lanes are never reaped automatically.

**D6 — Exit-code contract.** Exit codes are owned by the lane-state
layer — the overlap where a closed bead is simultaneously
`BeadOutcome::Completed` and `LaneState::AwaitingReview` resolves in
lane-state's favor. `abacus drain` exits 0 whenever the loop completes
without infrastructure error, regardless of class mix — the morning
report is the signal. `abacus run` exits 0 on the nominal settle — bead
closed, PR up, reviewer launched (`AwaitingReview`) or `Merged` — 3 for
a parked settle (`Blocked` / `Stalled`), and 1 for engine failure, so
wrappers distinguish parked-by-design from breakage. This amends the
TEST-STRATEGY gate proposal, which listed `AwaitingReview` in the
nonzero set before the review gate made it the nominal run outcome;
amendment presented for ratification at the RECORD gate.

**D7 — Extraction precedes new states.** `dispatch_cycle` (~140 inline
lines) is first decomposed behavior-preservingly into a lane-lifecycle
module, gated by the existing suite surviving by name and assertion body;
new states land only after.

**D8 — One module owns every grammar.** The BLOCKED token, the
adjudication heading and verdict grammar, the brief template, and the
status context string live as constants in a single module
(`src/review.rs` or a shared types seam); deployed repo contracts cite
them; a builder→parser round-trip test makes drift mechanical to catch.

## Test contract

TEST-STRATEGY (approved 2026-08-18): 28 new tests — 17 unit, 11
integration, zero new test files; real-br integration deliberately capped
at 2 (the comment-id seam); gh interactions covered by captured-fixture
parsers plus fake-shim forbidden-call assertions, never live GitHub; the
wedge's drain-continues test is red against pre-cluster HEAD by
construction. Measured baseline 9.5–12.9s against the 30s budget. The
known br_roundtrip flake (a br-side
`updated_at`-validation clock race, identified and jotted) gains a
message-matched retry in the test helper as part of this cluster.

## Consequences

Accepted: engine parsing is now coupled to two deployed textual
contracts (BLOCKED token, adjudication grammar) — mitigated by D8's
single-owner constants and round-trip test, and by the narrowness of
what is parsed. The monitoring guidance is evidence-backed for
codex-kind agents only; a claude-kind lane re-verifies settle vocabulary
first. Required-check enforcement varies per repo by GitHub plan;
onboarding records it as a precondition, and on unenforced repos the
gate is advisory — visible on the merge box, structural only where the
plan allows. Sweep-phase gh calls grow with live-lane count; accepted at
current scale (absorbing states are not re-probed), revisited only on
observed pressure. The operator's adjudication remains the sole
human-in-the-loop step of an overnight drain, by design.

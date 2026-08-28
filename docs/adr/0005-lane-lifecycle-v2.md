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
ground rules whose authorized deliverable is exactly one `gh pr comment`
write on the target PR, the field-calibrated correctness contract in D9,
and the required verdict grammar: `REFUTED` / `NOT REFUTED`, numbered
findings, Coverage, and Probes sections), writes it to a gitignored tmp
path in the target repo, and launches a fresh codex context in its own
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
with a git-worktree fallback). Warm author context is a cache, not durable
authority: retire it after roughly 10–12 correctness-review cycles or at
roughly 70% context consumption, whichever comes first, and earlier when
a design-escalation rule requires it. Retirement starts a fresh author
agent from durable bead and adjudication state in the surviving worktree,
branch, and PR; it does not reap or abandon the lane. These approximate
outer caps are lifecycle policy, not a new D1 engine state in this
amendment: context utilization is not currently a durable fact that the
stateless engine can reconstruct after a crash. Automatic enforcement
therefore requires a separately specified durable signal; the
orchestrating manual mode enforces the caps until one exists. Reaping
moves from settle to: `Merged` (force allowed, as today) or operator
abandon; `Blocked` lanes reap only when clean, inverting the existing
force path via the dirty-worktree error discrimination; `AwaitingReview`,
`ReworkRequested`, and `Stalled` lanes are never reaped automatically.

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

**D9 — The correctness contract incorporates the 2026-08-28 field
amendments as one coherent revision.** The dispositions below are
deliberately recorded together because separating them would make the
sweep, severity, and convergence clauses contradict one another:

1. **Phase-dependent enumeration — brief template.** At the first blocker,
   the reviewer asks whether its repair could plausibly moot other
   findings. A design-level or wrong-contract answer stops instance
   enumeration after two confirming executions and recommends a design
   pass; point defects on a stable design require an exhaustive sweep and
   complete list.
2. **Mandatory completeness — brief template and verdict grammar.** Every
   verdict names fully swept and unswept areas and explains exclusions in
   a `Coverage` section. A stopped design review is therefore visibly
   incomplete rather than falsely clean.
3. **Contract freeze — brief template.** After a blocker's core claim is
   guarded, residual precision hygiene on the new check replaces that
   check with the simplest sufficient contract or splits it out; the
   current PR never keeps extending the new check.
4. **Author rotation — D5.** Warm author context has the approximate
   10–12-cycle / 70%-context outer cap recorded above. No `src/lane.rs`
   mechanism lands with this amendment because the context threshold is
   neither exact nor durably observable; claiming crash-reconstructible
   automation would contradict D1.
5. **Focused gates — brief template.** Reviewers run the path-focused suite,
   verify import provenance where applicable, use targeted probes, and
   consume an explicit known-environment-issues list. They do not repeat
   the full suite by default; a broader gate needs a finding-specific
   reason recorded under Probes. Author and CI gates are unchanged.
6. **Security-surface framing — brief template.** Briefs use positive
   rejection-contract and correctness-invariant assertions throughout,
   preserving coverage while avoiding filter-sensitive framing.
7. **Guard relocation — brief template.** A guard-shaped finding names the
   narrowest choke point covering the whole class and probes a sibling;
   guard-shaped specifications receive the same “what else makes it pass?”
   interrogation before dispatch.
8. **Documentation-cited outcomes — brief template.** Byte evidence must
   demonstrate the claimed failure itself. A flag or serialization
   difference does not establish a downstream service outcome; that
   outcome must be executed on a representative real target or self-grade
   to concern.
9. **One-write purpose — brief template.** The permitted verdict comment is
   explicitly the reviewer's authorized deliverable, not merely an
   abstract permission.
10. **Verdict neutrality — brief template and manual-brief contract.** The
    sweep remains maximally adversarial, but a refuted verdict requires a
    genuinely serious defect; a clean verdict after a real sweep is a
    successful review, and effort never justifies escalating a minor
    issue. This supersedes the old convergence sentence.

The blocker floor remains an executed failure or byte-level demonstration
of the claimed failure on a realistic deployed path. The trusted-producer
calibration and cycle-two class rule remain unchanged. Exhaustive mode is
therefore not permission to pad or nitpick.

## Clarification 2026-08-27 — what the decisions above imply for an operator

Added after a measured comprehension failure, not as a change of decision.
An orchestrator reading this ADR cold on 2026-08-20 inferred an
acceptance-gate inversion and a one-shot review cycle from designed
behavior, and filed four notes against it. The decisions were correct; what
they imply was not stated anywhere an operator would look. Three
implications, all already entailed above:

- **A closed bead is an author-done signal, not an acceptance.** The worker
  closes when its own contract is satisfied. D6 already resolves the overlap
  in lane-state's favor — a closed bead whose lane is `AwaitingReview` is
  awaiting review, not accepted. Acceptance is the D4 adjudication comment
  plus, where configured, the required `adversarial-review` status at merge.
- **The adjudication gate is human-hand-posted and produces no prompt.** Per
  D4 the engine parses adjudications and never writes one. Per D2 the drain
  never blocks on the gate: it re-derives, finds no available transition,
  and exits 0. An operator unaware of the D4 grammar therefore leaves every
  lane waiting indefinitely while the drain keeps reporting success.
- **`run` and `drain` are different commands with different jobs.** D2 makes
  `drain` the loop that continues the ready front and performs review
  reconciliation; D6 makes `run` a single-dispatch settle whose nominal
  exit-0 outcome is `AwaitingReview`. A `run` that closes one bead and parks
  a lane awaiting review has succeeded, not truncated.

The operator-facing surface for these is `docs/lifecycle.md`. It also
records a standing conflict this ADR does not have authority to resolve:
`NORTH-STAR.md` states "a reviewer accepts, the bead closes", which
contradicts D3–D4 as deployed. Amending the north star is `/north-star`
revise mode — an explicit operator act.

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

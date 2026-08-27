```doc-meta
role: contract
lifecycle: active
```

# ADR 0006: The PR evaluator stack — two evaluators, one gate, manual-mode fallback

- **Status:** **proposed** 2026-08-27 — drafted at the `ab-ljn` RECORD
  gate; pending the design-document review gate (bloat review + spec
  validation) and operator disposition.
- **Date:** 2026-08-27
- **Deciders:** operator (all rulings), orchestrator session (record),
  with a Codex peer session as design counterparty (two-round evaluator
  discussion; it reversed its own position once on evidence) and the
  resident market-brief-package session as the field-evidence source.
- **Authority:** `NORTH-STAR.md` (kill criterion: machinery must not make
  execution slower than vanilla sessions; "review that yields too little
  leverage" is explicitly a defect to debug, not a kill signal); ADR 0005
  (lane lifecycle, review gate, adjudication grammar — related to, not
  amended, per D5 below); the operator rulings of 2026-08-27 recorded in
  `PLANNING-ab-ljn.md` (git history after handoff deletes it).

## Context

One adversarial reviewer gated every PR. Field evidence (market-brief-
package, 2026-08-26/27) showed its contract **structurally suppressed** a
class of true findings: "this is more complex than the problem needs" can
never clear an executed-failure bar, so shape findings were unsayable, not
merely crowded out. A hand-run second reviewer with an opposite contract
produced, in four runs, zero noise, two structural insights the
correctness reviewer never surfaced, and one restoration finding on a
reduction PR. Separately, the engine's review leg proved fragile enough
(`ab-645` wedged a drain; `ab-omt`, `ab-cye` open) that the operator ruled
manual orchestration the current mode and a permanent fallback thereafter.
This ADR binds the resulting conventions. The operating procedure lives in
`.claude/skills/abacus-execute/SKILL.md`; this document records only what
must survive skill rewrites.

## Decision

**D1 — The stack is two evaluators, each defined by epistemic mode, not
subject matter.** *Correctness*: does this work, and can a serious defect
be demonstrated — blockers, executed-failure evidence bar, threat models,
exhaustive sweep on stable designs, gates the merge. *Simplicity*: is this
the right shape for what the bead asked — proposals only, no severity
floor, no executed-failure requirement, speculation welcome, advisory. The
mode cut (posture, evidence bar, output, authority) is one decision
expressed four ways; the two contracts are deliberately opposite, and
neither is a reworded variant of the other. Admission of any further
evaluator requires one of exactly two routes: (a) **structural
suppression** — a finding class unsayable under every existing contract;
or (b) **demonstrated attention dilution** — repeated misses, recurring
operator pain, or a focused trial producing material findings the broad
evaluator overlooks. Route (b) is empirical; argument alone never admits.
Every evaluator is read-only, posts exactly one PR comment, and runs every
cycle unconditionally — an evaluator needing a launch predicate is out of
scope. Anything answerable from a plan or scope, or already covered by CI
or an existing evaluator, is not a PR evaluator.

**D2 — Exactly one gating evaluator.** Correctness owns the
`adversarial-review` commit status and merge authority; simplicity owns no
status and can never block a merge. A second gate would duplicate severity
decisions and deepen review arcs, and requires a genuinely separate
authority boundary plus a fresh operator ruling (extending ADR 0005's
cut-1 fence on check-flip authority). Advisory capacity is bounded by
attention, not machinery, and grows only through D1's admission routes.

**D3 — Heading registry and the collision rule.** Correctness posts
`## Adversarial review — cycle <n>` (ADR 0005 D4, unchanged). Simplicity
posts `## Simplicity review` — no cycle number, no verdict line. Hard
rule with stated consequence: **no evaluator other than correctness may
post a heading beginning `## Adversarial review — cycle `.** The engine's
heading parser prefix-matches and ignores trailing text; a colliding
heading registers a phantom verdict cycle that can kill a live correctness
reviewer, suppress its relaunch, and — if the phantom cycle is then
adjudicated — flip the required status and clear a PR to merge with zero
correctness review performed. A unit test in `src/review.rs` pins the
canonical simplicity heading as invisible to cycle bookkeeping; changing
either heading requires revisiting that test and this decision together.

**D4 — Adjudication binds the stack.** One adjudication comment per
correctness cycle, in ADR 0005 D4's byte-exact grammar, unchanged
byte-for-byte. The simplicity review is adjudicated **inside** that
comment as a labelled paragraph naming the disposition of its proposals;
it never receives its own `## Adjudication` heading or cycle number —
an adjudicatable simplicity verdict would be a de facto veto, contradicting
D2. Adjudication is a **transaction**: it states the reviewed head, the
surfaces examined, material exclusions, and a completeness judgement; and
every accepted concern receives its durable disposition in the same
operation — folded into the current rework, filed as a bead whose ID
appears in the adjudication, or explicitly rejected. An acceptance whose
filing is deferred is the documented mechanism by which concerns resurface
as later-cycle blockers.

**D5 — Two modes, one contract; manual mode is a permanent fallback.**
*Engine mode* is the destination: the engine launches, tracks, and reaps
evaluators, and hand-performing those actions remains forbidden there.
*Manual mode* is operator-declared — per repository or per session, never
inferred — and the orchestrator launches evaluators by the skill's written
procedure. Manual mode is never retired: an engine-mode defect halts
execution entirely, while under manual orchestration it is a minor
setback, so the fallback is a first-class operating mode. Binding
invariant: **mode equivalence at the comment stream** — both modes produce
byte-compatible PR artifacts (headings, verdict grammar, adjudication
grammar, cycle semantics), so the operator can switch modes mid-arc and
nothing strands. The advisory evaluator satisfies this by construction:
its heading is invisible to engine bookkeeping, so its comments are
engine-inert even when the engine runs. This ADR **relates to ADR 0005
without amending it** — manual mode operates outside D2/D3/D5's engine
loop, and nothing here changes what the engine parses. Making the engine
launch the second evaluator is future work under its own ADR 0005
amendment; the engine-side inventory exists at planning-record commit
`fb2c106`.

**D6 — Convergence controls govern cycle depth; no hard cycle cap.**
Review depth is controlled at its causes, not by a ceiling. A per-PR
**convergence ledger** — derived from the PR's verdict and adjudication
comments, never a repository-wide file — carries finding classes,
dispositions, regression probes, and enforcement seams; each cycle's brief
is generated from it, and a finding re-blocks only if its class is new or
a recorded regression is demonstrably live. On the **second** instance of
a class, rework moves the guard to the narrowest seam covering the class
and tests a sibling. On the **third**, the orchestrator stops rework, puts
the design question to the operator, and retires the warm author's
accumulated context (fresh agent, same worktree, branch, and PR). Rework
prompts are always regenerated from durable state, never "address cycle
N".

**D7 — The simplicity contract's core clauses are bound; its prose and
tactics are owned by the template.** The template file shipped beside the
skill is the single owner of the brief's wording. This ADR binds only the
clauses that carry operator rulings or field calibration: read-only
posture with one permitted comment plus `jot` capture for pre-existing
findings; the canonical heading per D3; the simplicity question (is this
the right shape for what the bead asked); the concepts-not-lines
objective, including that a change adding lines while removing a concept
is a valid simplification and that a simplification requiring an
existing-test change is a behaviour change to report, not propose; the
narrow test clause (changed or behaviour-implicated tests only; unit is
behaviours and assertions — duplicate behavioural proof, assertions no
longer pinning the named contract, coverage lost through folding,
deletion, or thinning; never a suite inventory); and the proposal shape
(what is removed; which guarantee survives and how it checked; rough
cost) with a mandatory considered-and-rejected section — the field's
calibration signal. Further tactics — reduction-PR probes, volume and
ranking rules, exclusion lists, framing vocabulary — are template-owned
and may evolve without revisiting this decision.

**D8 — The test-focus mandate is trialled inside simplicity; promotion is
evidence-gated.** Test bloat is an observed problem in the autonomous
execution layer, but its origin (bead spec versus worker behaviour) is
unestablished and a dedicated evaluator is the most expensive recurring
instrument for it. The narrow test clause (D7) carries the mandate now. A
shadow trial — a one-off, non-stack test specialist inspecting 5-8
representative historical diffs against the instructed simplicity
reviewer's output, with each observed excess classified spec-originated or
worker-originated — decides promotion. Threshold: two independent PRs
where the shadow finds material test defects the instructed simplicity
reviewer missed, especially same-class misses. If bloat instead traces to
bead specs or author contracts, those are fixed and no evaluator is
added.

## Consequences

Review cost roughly doubles in tokens per cycle and stays flat in
wall-clock (parallel launch); the operator reads two reports and disposes
of simplicity proposals inside the existing adjudication act. The
correctness gate is unchanged in authority and grammar, so nothing
downstream of merge changes. The heading rule converts the one identified
catastrophic interaction (phantom cycle → false acceptance) from a silent
hazard into a pinned, tested constraint. Manual mode's legitimacy makes
engine defects non-blocking at the cost of orchestrator labour per cycle;
that cost is the standing argument for finishing engine mode, and D5's
equivalence invariant keeps that migration a switch rather than a rework.
The admission rules make the stack's growth deliberately hard: the next
evaluator arrives with evidence or not at all.

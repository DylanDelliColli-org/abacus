```doc-meta
role: contract
lifecycle: active
```

# ADR 0007: Three-seat evaluator stack, and reviewer model/effort selection

- **Status:** **proposed** 2026-09-01 — drafted at the operator's stack-lock
  confirmation. Review trail: bloat review (fresh Codex context, seven
  cuts, operator-disposed — four applied, one modified, two rejected with
  the operator's reasons) and spec validation (second fresh Codex context,
  four findings, all applied as faithfulness fixes same day).
- **Date:** 2026-09-01
- **Deciders:** operator (the lock, the promotion, both mid-experiment
  hypotheses that redirected the program), orchestrator session (record and
  E-checks).
- **Authority:** ADR 0006 (the two-seat stack, its admission routes, and the
  D8 evidence-gated promotion path — this ADR is that path exercised, not an
  amendment); `NORTH-STAR.md` kill criterion; the observation record
  `docs/compatibility/2026-08-31-reviewer-model-selection-experiment.md`,
  which carries the complete data and is the citable justification for every
  decision below.

## Context

ADR 0006 shipped a two-seat stack and bound a trial deciding whether
test-focus review is promoted to a third seat. The trial met its promotion
threshold five times over. In parallel, the operator hypothesised that
Sol-tier maximum reasoning was not required for every seat; a seven-wave,
30-context experiment (the observation record above) established that most
of the capability delta at cheaper configs was prompt-shaped — closed by
prescribing execution *methodology* rather than only an evidence bar — and
a final wave validated the full three-seat stack at the cheaper config
against banked ground truth. The operator's resulting thesis, which the
data supports on its sample: more reviewers with narrower scopes at lower
effort, methodology-briefed, with full adjudicated-blocker recall on the
tested sample at materially lower configured effort and measured
wall-clock.

## Decision

**D1 — The stack is three evaluators.** *Correctness* (gating, owns
`adversarial-review`, unchanged); *simplicity* (advisory, unchanged except
D3's carve); *test-quality* (advisory, promoted via ADR 0006 D8 on trial
evidence). The test-quality evaluator answers exactly: does every changed
or behaviour-implicated test buy a distinct, load-bearing behavioural
guarantee at proportionate cost, and does the suite still prove every live
behaviour the diff touches. Heading: `## Test-quality review` — first body
line, no cycle number, no verdict line; engine-inert by the ADR 0006 D3
collision rule. Adjudicated inside the correctness adjudication comment as
a labelled paragraph beginning "The parallel test-quality review is
adjudicated separately and does not gate this merge", placed alongside the
simplicity paragraph — after the finding lines, before the Adjudicated
head line — so manual and engine artifacts stay byte-compatible.

**D2 — Methodology sections are canonical brief content, every seat, every
tier.** The experiment's central finding: an evidence *bar* does not elicit
the executed *method*. The correctness brief carries the fake-shim
execution methodology (build the reviewed binary; PATH-first fake
`br`/`herdr`/`gh` logging argv; scenarios derived from bead claims;
findings graded on observed traces). The simplicity brief carries
concept-to-guarantee tracing, the claims-vs-tree audit with parent diffs on
merge commits, and execute-don't-estimate. The test-quality brief carries
the six-method procedure exercised in wave 7 (behaviour map, distinctness
map, label-honesty audit, reachability check, deleted-assertion accounting,
unconditional measurement). Methodology is a precondition of D3's model
selection, not a preference.

**D3 — Scope carve between the seats.** Test-suite design (duplicate
proof, coverage loss, label honesty, cost) belongs to the test-quality
seat. Simplicity's test responsibility narrows to incidental one-line
observations. Correctness retains test verification wherever tests
substantiate the bead's claims — the guard-neutralization class stays at
the gate. A suspicion in another seat's territory is noted in one line and
not developed.

**D4 — Model and effort selection.** All three seats run `gpt-5.6-sol` at
`medium` reasoning with methodology briefs, selected per launch via the
herdr `AGENT_ARG` passthrough. Basis: full adjudicated-blocker recall and zero manufactured
findings across the program (record, correctness arm and wave 7).
`gpt-5.6-luna` is disqualified for standing seats (model-shaped recall
floor on engine-internals targets at both tested efforts).

**D5 — The missed-blocker tripwire.** The small-n caveats in the
observation record are corrected by observation, not insurance: any
post-merge defect that review should have caught triggers a config
re-evaluation for the affected seat. This is the MVP-first ruling applied to model selection.

**D6 — The evidence record is binding context.** The observation record in
`docs/compatibility/` is the justification of record for D1–D5. Future
sessions revisiting these decisions read it first; a proposal to change
seat count, scope, or config engages its data or supersedes it with new
measurement, never argues past it.

## Consequences

Per-cycle review cost drops materially in measured wall-clock and in
configured reasoning effort while seat count rises; the operator reads three reports and disposes of two advisory
paragraphs inside the existing adjudication act. The known residuals ride
on the record: the test seat's deepest enumeration trails the Sol-tier
shadow benchmark on the hardest target, and every conclusion is small-n
until real drains accumulate.

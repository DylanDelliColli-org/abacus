```doc-meta
role: contract
lifecycle: active
```

# ADR 0007: Three-seat evaluator stack, and reviewer model/effort selection

- **Status:** **proposed** 2026-09-01 — drafted at the operator's stack-lock
  confirmation; pending the design-document review gate.
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
effort, methodology-briefed, at equal-or-better detection and a fraction of
cost.

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
a second labelled paragraph, symmetric with simplicity's. ADR 0006's
admission rules and single-gate rule stand for any fourth seat.

**D2 — Methodology sections are canonical brief content, every seat, every
tier.** The experiment's central finding: an evidence *bar* does not elicit
the executed *method*. The correctness brief carries the fake-shim
execution methodology (build the reviewed binary; PATH-first fake
`br`/`herdr`/`gh` logging argv; scenarios derived from bead claims;
findings graded on observed traces). The simplicity brief carries
concept-to-guarantee tracing, the claims-vs-tree audit with parent diffs on
merge commits, and execute-don't-estimate. The test-quality brief carries
the six-method procedure validated in wave 7 (behaviour map, distinctness
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
herdr `AGENT_ARG` passthrough. Basis: full adjudicated-blocker recall on
every head tested, one novel true positive the Sol-tier baseline missed,
zero manufactured findings at any tier in the program, and a full-stack
wall-clock of 9m25s versus 10–20 minutes for a single Sol-tier review.
`gpt-5.6-luna` is disqualified for standing seats (model-shaped recall
floor on engine-internals targets at any effort); it may serve as a
supplementary diverse lens under ADR 0006's admission rules only.

**D5 — The missed-blocker tripwire.** The small-n caveats in the
observation record are corrected by observation, not insurance: any
post-merge defect that review should have caught triggers a config
re-evaluation for the affected seat (first candidate: Sol-high, the
untested middle). This is the MVP-first ruling applied to model selection.

**D6 — The evidence record is binding context.** The observation record in
`docs/compatibility/` is the justification of record for D1–D5. Future
sessions revisiting these decisions read it first; a proposal to change
seat count, scope, or config engages its data or supersedes it with new
measurement, never argues past it.

## Consequences

Per-cycle review cost drops materially in tokens and wall-clock while seat
count rises; the operator reads three reports and disposes of two advisory
paragraphs inside the existing adjudication act. The known residuals ride
on the record: the test seat's deepest enumeration trails the Sol-tier
shadow benchmark on the hardest target, and every conclusion is small-n
until real drains accumulate. The contract work this ADR implies — the
test-quality brief as a shipped template, the methodology sections landed
in the canonical briefs, and the heading-registry addition — is tracked as
implementation beads under the `ab-ljn` epic's successor work, not
performed by this document.

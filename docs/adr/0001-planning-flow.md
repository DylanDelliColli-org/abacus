# ADR 0001: The planning flow enters ABACUS as a skill

- **Status:** **accepted** 2026-08-14, operator approval after both review
  passes. Trail: bloat review by a fresh Codex context proposed six cuts —
  operator applied one (producer mandates softened to defaults) and
  reaffirmed five; spec validation by a second fresh Codex context produced
  five anchored findings, all applied as clarifications (flow-level success
  bar defined, quick-tier classifier restored, open-questions rule scoped to
  both tiers, research outputs marked provisional with re-derivation at
  DECOMPOSITION, budget pricing marked as estimates).
- **Date:** 2026-08-14
- **Deciders:** operator (direction), orchestrator session (record)
- **Authority:** NORTH-STAR.md thesis as amended 2026-08-14 ("a
  human-in-the-loop planning flow turns operator intent into a well-defined
  backlog"); amendment log entry of the same date.

## Context

The execution half of ABACUS works: dispatch, outcome verification from bead
state, lane reaping, and worker-opened PRs ran end to end across eight worker
lanes on 2026-08-13/14. The backlog that feeds it, however, is still authored
by hand or by `sable-plan` — a flow written for substrate this machine has
retired: the `bd` tracker (replaced by `br`), the SABLE interlock hooks
(removed 2026-08-13), and the Optimus/Tarzan/Chuck manager fleet (replaced by
`abacus run`).

The operator has directed bringing the planning flow into the product. This
record captures how, and what is deliberately not being built.

## Decision

### 1. Skill first, machinery after evidence

Planning enters as a skill at `.claude/skills/abacus-plan/` in this
repository, adapted from `sable-plan`. The engine binary does not change now.
Engine support (for example, dispatching planning producers as lanes) is
added only where running the skill demonstrably hurts, one observed pain
point at a time.

The SABLE interlock is not rebuilt. The skill states its gates as rules; a
gate that gets skipped in practice is the evidence that buys enforcement,
and not before.

### 2. The flow: two tiers, six substages

Tier sizing survives from `sable-plan` unchanged, including its classifier:
**quick** requires a well-specified ask with no unknowns to research, no new
interface or contract, and a bounded blast radius (roughly 1–3 beads, one
consolidated approval). Anything with unknowns, architecture decisions, or a
wide radius is **full**, and doubt defaults to full. The skill proposes a
tier; the operator confirms.

The full tier runs six gated substages — five inherited, one new. Each
substage names a **default producer**; the substages and their deliverables
are the contract, and the skill may substitute any capable producer for a
default. Only the operator gates are fixed choreography.

| Substage | Default producer | Deliverable |
|---|---|---|
| FRAMING | orchestrator with the operator, live | stories, non-goals, success metric, wedge, prerequisites |
| RESEARCH | sherlock-type subagent | prior art, pitfalls, **module fingerprints, candidate bundle groups** |
| ARCHITECTURE | gaudi skill, inline | locked interface and design decisions |
| TEST-STRATEGY | columbo-type subagent | story-by-test matrix **priced against the suite budget** |
| RECORD *(new)* | orchestrator | ADR (common) or PRD (rare) into `docs/` |
| DECOMPOSITION | orchestrator + victor-type subagent | Fresh-Agent-Test-clean children with footprints and group tags |

Each substage ends at an operator gate. The operator signs off on the
deliverable before the flow advances. The quick tier skips RECORD along with
the rest of the ceremony.

Substage deliverables are **committed to the repository** (not gitignored, as
SABLE's were) under a planning state directory per epic. Crash recovery is
first-class on this host (CONSTRAINTS.md finding 4): a planning session must
be resumable from the tree alone.

### 3. RECORD: paper documentation with a gate

A full-tier run usually warrants an ADR; occasionally a PRD. RECORD sits
after TEST-STRATEGY because that is the first moment all upstream substance
exists to cite — framing, research findings, locked architecture, and the
test contract. Beads then decompose from the record, not from conversation
memory.

The existing design-doc review gate (installed in both agent configs) fires
automatically on ADR/PRD creation and injects the bloat-review and
spec-validation discipline. That gate is what keeps this from repeating the
v1 failure mode of documents accreting as authority: every record must
survive an adversarial "is this needed right now" pass, and the flow it
belongs to ends in dispatched beads, not in the document.

### 4. Operator-facing prose rules

All operator-facing output of the skill — questions, gate summaries,
deliverable prose — follows three rules:

1. Use Google's standards for technical writing.
2. Explain in simple terms without losing technical precision.
3. A bead referenced by ID must be explained in one sentence at the point of
   reference.

Rules 1 and 2 live in this skill. Rule 3 starts here and graduates to the
global agent instructions once it has proven its worth in practice.

### 5. Test selectivity with a visible budget

SABLE treated coverage as free and its pre-push gate grew to roughly 45
minutes. ABACUS makes test cost visible and budgeted:

- The full suite of this repository has a wall-clock budget of **30
  seconds**, stated in the skill as a constant. Raising it requires an
  argued change, not drift.
- At TEST-STRATEGY, every proposed test addition states its **estimated**
  runtime cost against the remaining budget — measured costs exist only once
  the tests do, and the periodic audits below use measured durations. A
  threatened budget forces choosing, which is the selectivity working as
  intended.
- Bias to extending existing tests over adding files; integration tests sit
  at real seams (where two systems meet), not on every story.
- Periodic cost audits rank tests by duration against unique coverage
  contributed and propose pruning.
- The three coverage-loss tripwires (folded cases, deleted tests for live
  code, assertions thinned inside surviving test names) are the
  DECOMPOSITION checklist guard against over-pruning.

### 6. Fingerprints and bundle groups at RESEARCH

RESEARCH produces module fingerprints while the backlog is still small —
retrofitting fingerprints onto a large backlog was expensive in SABLE, and
doing it early is the fix. Because research develops a provisional map of
which files each future bead is likely to touch, it also proposes
**candidate bundle groups**: small, footprint-overlapping beads that would
block each other as separate lanes and should instead go to one worker as
one lane and one PR.

Research fingerprints and groups are **provisional**: ARCHITECTURE may
change the design they assumed. DECOMPOSITION re-derives every child's final
file footprint from the locked architecture, applies a group tag only where
the candidate grouping survives that re-derivation, and drops stale
candidates rather than stamping them.

DECOMPOSITION thus stamps every child with its final footprint and, where
one survived, a group tag. The skill records groups; the engine
learns to dispatch them later — bundled dispatch is machinery, and machinery
follows evidence (the two-lane stress test produces the first measurements
of cross-lane conflict cost).

### 7. Handoff

A completed planning run hands off to `abacus run`. The open-questions rule
survives from `sable-plan` with its original scope — **both tiers**:
ambiguity surfaced at any point in a quick or full run becomes an
operator-addressed question, and the flow does not hand off while open
questions remain.

### 8. The flow's success bar

Per the north-star amendment, the planning flow's success bar is defined
here and promoted into NORTH-STAR.md only with evidence. A planning run
**succeeds** when the backlog it hands off drains through `abacus run`
without any worker lane stopping for missing scope or an operator answer —
zero open questions at handoff, and zero scope-starved lanes during the
drain. FRAMING's per-epic success metric measures the epic's outcome; this
bar measures the flow itself, and the two are not substitutes.

## Consequences

- The backlog-authoring half of the amended thesis becomes real without any
  engine change, and without rebuilding retired enforcement machinery.
- Planning state in the tree makes sessions crash-resumable and
  fresh-agent-readable, at the cost of planning commits in history.
- The 30-second suite budget will eventually force a genuinely hard
  coverage choice; that pressure is the design, not a defect.
- The engine's future planning features (producer lanes, bundled dispatch)
  now have a defined place to grow from, each gated on observed need.

## Not built now

No interlock or enforcement hooks. No dossier HTML tooling. No engine
subcommand. No producer-as-lane dispatch. No bundled dispatch in the engine.
Each waits for its observed failure or measured need.

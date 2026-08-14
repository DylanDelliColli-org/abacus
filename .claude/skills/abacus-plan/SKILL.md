---
name: abacus-plan
description: Turn operator intent into an execution-ready br backlog with the human-in-the-loop ABACUS planning flow. Use when asked to run /abacus-plan, start planning, choose a Quick or Full planning tier, fill the backlog, or prepare work for abacus run.
---

# /abacus-plan — turn intent into an execution-ready backlog

Plan so execution becomes boring. Keep every load-bearing decision with the
operator, or surface it as an explicit, vetoable assumption before creating
implementation work. Produce a Fresh-Agent-Test-clean `br` backlog whose workers
can execute without recovering scope from conversation history.

Planning is a skill, not engine machinery. Follow the flow below directly. Do
not add enforcement, hooks, or engine behavior to compensate for a skipped step;
an observed failure is the evidence required before proposing machinery.

## Operator-facing prose

Apply these rules to every question, gate summary, and deliverable:

1. Use Google's standards for technical writing.
2. Explain in simple terms without losing technical precision.
3. A bead referenced by ID must be explained in one sentence at the point of reference.

## Constants

`FULL_SUITE_WALL_CLOCK_BUDGET_SECONDS = 30`

Treat this as the repository full-suite wall-clock budget. Raising it requires
an argued change; do not let it drift. During planning, price proposed tests with
**estimated** runtime because measurements do not exist until the tests run.
Treat later timings as **measured** runtime, never as retroactive proof that the
estimate was measured.

## 1. Size the ask and establish durable state

Read the ask, recommend a tier, explain the classification, and ask the operator
to confirm it. Never select the tier unilaterally.

- Recommend **Quick** only when all of these are true: the ask is
  well-specified, there are no unknowns to research, it introduces no new
  interface or contract, and its blast radius is bounded to roughly 1–3 beads
  with one consolidated approval.
- Recommend **Full** for any unknown to de-risk, architecture decision to lock,
  or wide blast radius. When in doubt, recommend Full.

Identify or create the planning epic before producing artifacts. Use
`docs/planning/<epic-id>/` as its state directory and commit planning state to
the repository. A session must be resumable from the tree alone after a host
crash; do not rely on chat history or ignored local state, and never gitignore
these deliverables.

Use one Markdown file per deliverable. A Full run uses:

```text
docs/planning/<epic-id>/
├── framing.md
├── research.md
├── architecture.md
├── test-strategy.md
├── record.md
└── decomposition.md
```

A Quick run uses one consolidated `quick.md` in the same directory. Put the
confirmed tier and its rationale at the top of the first artifact.

## 2. Use the gate protocol

For every Full substage:

1. Read the approved upstream files from the planning state directory.
2. Have the default producer create the required deliverable, or substitute any
   capable producer when that is more effective. Record a material substitution
   in the deliverable.
3. Check the file for unresolved assumptions, operator-addressed questions, and
   the required traceability to upstream decisions.
4. Commit the reviewable deliverable. If the operator requests changes, revise
   and commit it again.
5. Summarize the decision in operator-facing prose and ask for signoff.
6. Advance only after the operator approves that substage.

The substages and their deliverables are fixed. Producer identities are
defaults and are substitutable. The operator gate at the end of every substage
is fixed choreography and is never substitutable.

## Quick tier — one lightweight pass

Skip the six Full substages, including RECORD. Use one lightweight pass and one
consolidated operator gate:

1. State the intended outcome, narrow scope, non-goals, acceptance conditions,
   and any prerequisites.
2. Specify the unit and integration test delta, biased toward extending existing
   tests. For a pure documentation or configuration change with no code path,
   mark and justify `[no-test]` instead. Price the estimated full-suite effect
   against `FULL_SUITE_WALL_CLOCK_BUDGET_SECONDS`.
3. Draft 1–3 implementation beads. Give each enough context, file paths,
   acceptance scenarios, test assertions, and verification commands for a fresh
   agent to act without reconstructing the plan.
4. Write and commit `quick.md` with the frame, test strategy, proposed beads,
   confirmed prerequisites, tier rationale, and open-question status.
5. Present that file and the proposed bead scope for one consolidated operator
   approval. Create or finalize the `br` children only within that approved
   scope.

If a real unknown or interface decision appears, stop and offer to move to Full.
Never silently retain Quick or downgrade rigor. Do not hand off until every open
question is resolved.

## Full tier — six gated substages

Run these substages in order. End each one at its operator gate before starting
the next.

| Substage | Default producer | Committed state deliverable |
|---|---|---|
| FRAMING | orchestrator with the operator, live | `framing.md` |
| RESEARCH | sherlock-type subagent | `research.md` |
| ARCHITECTURE | gaudi skill, inline | `architecture.md` |
| TEST-STRATEGY | columbo-type subagent | `test-strategy.md` |
| RECORD | orchestrator | `record.md` plus an ADR or, rarely, a PRD under `docs/` |
| DECOMPOSITION | orchestrator with a victor-type subagent | `decomposition.md` |

### FRAMING

Work live with the operator. Define:

- user stories with stable identifiers for later traceability;
- non-goals;
- one epic success metric;
- the narrowest valuable wedge; and
- prerequisites, using exact bead IDs or an explicit statement that there are
  none.

Write `framing.md`, commit it, and obtain operator approval before RESEARCH.

### RESEARCH

Research prior art, domain pitfalls, and unknowns that could invalidate the
frame. Produce provisional module fingerprints for likely implementation
areas. Anchor each fingerprint to repository evidence such as paths, symbols,
seams, or verification commands, and state its confidence.

Propose candidate bundle groups where small future beads appear likely to have
overlapping file footprints and would block each other as separate lanes. For
each candidate, state the predicted overlap and why one lane and one PR may be
preferable. Mark all fingerprints and bundle groups **provisional** because the
architecture is not locked yet.

Write `research.md`, commit it, and obtain operator approval before
ARCHITECTURE.

### ARCHITECTURE

Lock the interfaces, contracts, design decisions, and system tradeoffs needed by
the approved frame. Resolve the researched unknowns that affect the design and
identify smell or migration risks. State which research assumptions changed;
do not silently carry provisional findings forward as decisions.

Write `architecture.md`, commit it, and obtain operator approval before
TEST-STRATEGY.

### TEST-STRATEGY

Build a story-by-test matrix from the stable story identifiers and locked
architecture. Cover boundary cases and failure modes with the smallest useful
unit and integration delta. Bias toward extending existing tests. Put
integration coverage at real seams where two systems meet, not on every story.

For every proposed addition, record the test layer, intended assertion, whether
it extends an existing test, and its estimated runtime cost. Sum those estimates
against the remaining `FULL_SUITE_WALL_CLOCK_BUDGET_SECONDS` budget. If the
budget is threatened, choose the tests with the greatest unique confidence
rather than silently increasing the budget.

Use measured durations only after implementation. In periodic test-cost audits,
rank measured duration against unique coverage contributed and propose pruning.
Protect against these three coverage-loss tripwires during DECOMPOSITION:

- cases folded together until distinct behavior is no longer proved;
- tests deleted while their production behavior remains live; and
- assertions thinned inside surviving test names.

Write `test-strategy.md`, commit it, and obtain operator approval before RECORD.

### RECORD

Create an ADR in the common case or a PRD in the rare case where product
requirements, rather than a design decision, are the durable authority. Put the
record under `docs/` and make it cite the approved framing, research, locked
architecture, and test contract. Let the configured design-document review run;
resolve its bloat-review and specification-validation findings before the gate.

Write `record.md` with the durable record's path, why ADR or PRD was selected,
its review evidence, and its final decision summary. Commit both files and
obtain operator approval before DECOMPOSITION. The backlog must derive from this
record, not from conversation memory.

### DECOMPOSITION

Re-derive every child's final file footprint from the locked architecture. Do
not copy RESEARCH fingerprints forward mechanically. Compare the re-derived
footprints with the provisional bundle groups, retain only groupings that still
overlap, and drop every stale candidate. Where a grouping survives, add the same
explicit group tag to each member and record that tag in `decomposition.md`.
Record groups only; do not claim the engine already dispatches them as bundles.

Author implementation children under the epic with `br`. Make every child pass
the Fresh Agent Test: a fresh worker should be able to act from the bead and
repository alone. Include the originating story and acceptance scenario, exact
paths and relevant symbols, the decided approach and gotchas, unit and
integration test assertions (or a justified `[no-test]`), verification commands,
and dependencies stated as requirements.

Every child description must contain this dedicated section:

```markdown
## File footprint

path/to/file.rs, tests/relevant_test.rs
```

Use final planner-declared write paths, including files the child will create.
Where a bundle candidate survives, also attach its shared group tag to every
member, for example as the same `br` label. Keep unrelated footprints separate.

After creating the children, inspect `br dep tree <epic-id>`, `br ready`, and
`br lint`. Confirm that dependency direction matches requirement language,
intentionally blocked work is not ready, all footprints are final, and each
surviving group has a consistent tag. Have the freshness producer review the
beads without relying on planning conversation.

Write `decomposition.md` with child IDs and titles, story traceability, final
footprints, dependency and ready-state checks, retained group tags, dropped
candidate groups, and the freshness verdict. Commit it and obtain the final
operator approval.

## Open questions — both tiers

Turn every ambiguity discovered in Quick or in any Full substage into an
operator-addressed question. Track it durably on the epic and, when it needs its
own work-state item, create a `br` bead labeled `open-question`; make affected
children depend on it. Record the answer and close the question only after the
operator decides it.

Do not hand off either tier while any open question remains. An unanswered
question is not a planner assumption and an empty prerequisites list is not a
substitute for asking.

## Hand off to execution

After the tier's required gate or gates are approved, verify that planning files
and bead state are committed, `br lint` is clean, the ready front matches the
approved plan, and there are zero open questions. Tell the operator to run:

```bash
abacus run
```

Do not launch execution as an implicit planning step.

Judge the planning flow by its own success bar: a planning run succeeds only
when the backlog handed to `abacus run` drains with **zero open questions at
handoff** and **zero worker lanes stopping for missing scope or an operator
answer**. Record both observations after the drain. FRAMING's epic success
metric measures the product outcome; this success bar measures the planning
flow, and neither substitutes for the other.

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

Identify or create the planning epic before producing artifacts, and record
planning state by lifecycle (ADR 0001, amendment 2026-08-14). A session must
be resumable from the tree alone after a host crash; do not rely on chat
history or ignored local state.

- A **Quick** run records its consolidated state on the epic bead itself
  (`br update` — description or notes). It creates no planning files or
  directories.
- A **Full** run keeps exactly one committed in-flight record at the
  repository root: `PLANNING-<epic-id>.md`, registered under the corpus
  `inflight_globs`. Each substage appends its section — FRAMING, RESEARCH,
  ARCHITECTURE, TEST-STRATEGY, RECORD, DECOMPOSITION — and the file is
  committed at every gate. Put the confirmed tier and its rationale at the
  top.

The in-flight record is working state, not an archive: it is deleted at
handoff (see below); git history is the archive.

## 2. Use the gate protocol

For every Full substage:

1. Read the approved upstream sections of the in-flight record.
2. Have the default producer create the required deliverable, or substitute any
   capable producer when that is more effective. Record a material substitution
   in the deliverable. End every subagent dispatch prompt with an explicit
   instruction to send the finished deliverable back as a message before going
   idle — spawned producers otherwise idle silently and each one costs a nudge
   round-trip (observed 3/3 on 2026-08-17).
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
2. Specify the unit and integration test delta with this method: grep the
   existing test surface for the touched components first and bias toward
   extending named existing tests; include negative-space cases — what must
   NOT appear or happen; give every case concrete inputs and expected
   assertions, never "edge case". For a pure documentation or configuration
   change with no code path, mark and justify `[no-test]` instead. Price the
   estimated full-suite effect against `FULL_SUITE_WALL_CLOCK_BUDGET_SECONDS`.
3. Draft 1–3 implementation beads. Give each enough context, file paths,
   acceptance scenarios, test assertions, and verification commands for a fresh
   agent to act without reconstructing the plan.
4. Record on the epic bead (`br update` — description or notes) the frame,
   test strategy, proposed beads, confirmed prerequisites, tier rationale,
   and open-question status. The tracker's committed JSONL is the durable
   state; create no files.
5. Present that record and the proposed bead scope for one consolidated
   operator approval. Create or finalize the `br` children only within that
   approved scope.

If a real unknown or interface decision appears, stop and offer to move to Full.
Never silently retain Quick or downgrade rigor. Do not hand off until every open
question is resolved.

## Full tier — six gated substages

Run these substages in order. End each one at its operator gate before starting
the next.

| Substage | Default producer | Committed state deliverable |
|---|---|---|
| FRAMING | orchestrator with the operator, live | FRAMING section of the in-flight record |
| RESEARCH | sherlock-type subagent | RESEARCH section |
| ARCHITECTURE | gaudi skill, inline | ARCHITECTURE section |
| TEST-STRATEGY | columbo-type subagent | TEST-STRATEGY section |
| RECORD | orchestrator | RECORD section, plus an ADR in `docs/adr/` (or rarely a PRD) only when warranted |
| DECOMPOSITION | orchestrator with a victor-type subagent | DECOMPOSITION section |

### FRAMING

Work live with the operator. Define:

- user stories with stable identifiers for later traceability;
- non-goals;
- one epic success metric;
- the narrowest valuable wedge; and
- prerequisites, using exact bead IDs or an explicit statement that there are
  none.

Append the FRAMING section to `PLANNING-<epic-id>.md`, commit it, and obtain
operator approval before RESEARCH.

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

Append the RESEARCH section to `PLANNING-<epic-id>.md`, commit it, and obtain
operator approval before ARCHITECTURE.

### ARCHITECTURE

Lock the interfaces, contracts, design decisions, and system tradeoffs needed by
the approved frame. Resolve the researched unknowns that affect the design and
identify smell or migration risks. State which research assumptions changed;
do not silently carry provisional findings forward as decisions.

Append the ARCHITECTURE section to `PLANNING-<epic-id>.md`, commit it, and
obtain operator approval before TEST-STRATEGY.

### TEST-STRATEGY

Build a story-by-test matrix from the stable story identifiers and locked
architecture. Cover boundary cases and failure modes with the smallest useful
unit and integration delta. Bias toward extending existing tests. Put
integration coverage at real seams where two systems meet, not on every story.

For every proposed addition, record the test layer, intended assertion, whether
it extends an existing test, and its estimated runtime cost. Compute the
remaining budget by timing the suite that already exists — remaining equals
`FULL_SUITE_WALL_CLOCK_BUDGET_SECONDS` minus the current measured full-suite
duration; only tests that do not yet exist are estimated. Sum the estimates
against that remaining budget. If the
budget is threatened, choose the tests with the greatest unique confidence
rather than silently increasing the budget.

Use measured durations only after implementation. In periodic test-cost audits,
rank measured duration against unique coverage contributed and propose pruning.

Append the TEST-STRATEGY section to `PLANNING-<epic-id>.md`, commit it, and
obtain operator approval before RECORD.

### RECORD

RECORD is **artifact-conditional**: the substage and its operator gate always
run, but the durable document is produced only when the run locked a durable
decision or requirement someone will need to consult later — an ADR in the
common case, a PRD in the rare case where product requirements are the
authority. When no document is warranted, RECORD's deliverable is simply its
section of the in-flight record.

When a document is produced, path placement is load-bearing: the
design-document review gate triggers only on files under `docs/adr/` or
filenames carrying `adr`, `prd`, or a `proposal` prefix. Put an ADR in
`docs/adr/`; give a PRD a filename containing `prd`. A record elsewhere under
`docs/` silently skips the gate. Make it cite the approved framing, research,
locked architecture, and test contract, and resolve the gate's bloat-review
and specification-validation findings before signoff — their absence means
the gate did not fire, not that the record passed.

Append the RECORD section to `PLANNING-<epic-id>.md` — the document's path
and review evidence when one was produced, or the decision that none was
warranted. Commit and obtain operator approval before DECOMPOSITION.

### DECOMPOSITION

Re-derive every child's final file footprint from the locked architecture. Do
not copy RESEARCH fingerprints forward mechanically. Compare the re-derived
footprints with the provisional bundle groups, retain only groupings that still
overlap, and drop every stale candidate. Where a grouping survives, add the same
explicit group tag to each member and record that tag in the DECOMPOSITION
section. Record groups only; do not claim the engine already dispatches them
as bundles.

Author implementation children under the epic with `br`, deriving them from
the **approved planning state** — the ADR or PRD when one exists, otherwise
the gated sections of the in-flight record — never from conversation memory.
Make every child pass
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
surviving group has a consistent tag. Check every child's test spec against
the three coverage-loss tripwires:

- cases folded together until distinct behavior is no longer proved;
- tests deleted while their production behavior remains live; and
- assertions thinned inside surviving test names.

Also check every child's acceptance criteria for self-negating sweep
clauses: an acceptance phrased as grep-to-zero — or any
remove-every-occurrence order — must scope out test literals and
protected assertions explicitly, or a worker satisfying acceptance
deletes the very coverage the spec protects.

Have the freshness producer review the beads without relying on planning
conversation.

Append the DECOMPOSITION section to `PLANNING-<epic-id>.md` with child IDs
and titles, story traceability, final footprints, dependency and ready-state
checks, retained group tags, dropped candidate groups, the tripwire verdict,
and the freshness verdict. Commit it and obtain the final operator approval.

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

After the tier's required gate or gates are approved, verify that planning
state and bead state are committed, `br lint` is clean, the ready front
matches the approved plan, and there are zero open questions. For a Full run,
complete the handoff by deleting `PLANNING-<epic-id>.md` from the tree in the
handoff commit — its durable substance now lives in the beads, the epic, and
any RECORD document; git history is the archive. A Quick run has nothing to
delete. Tell the operator to run:

```bash
abacus run
```

Do not launch execution as an implicit planning step.

Judge the planning flow by its own success bar: a planning run succeeds only
when the backlog handed to `abacus run` drains with **zero open questions at
handoff** and **zero worker lanes stopping for missing scope or an operator
answer**. After the drain, record both observations as `br` notes on the
epic — the drain may finish in a different session than the one that planned,
and the epic's tracker record is the artifact the promotion-with-evidence
path cites. FRAMING's epic success metric measures the product outcome; this
success bar measures the planning flow, and neither substitutes for the
other.

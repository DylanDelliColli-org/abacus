```doc-meta
role: contract
lifecycle: active
```

# ABACUS — North Star

Established 2026-08-12 by operator interview (`/north-star`, establish mode).
Amendments happen only through revise mode, never as a consequence of an
inconvenient check. The amendment log is at the end of this document.

This document states the goal. It is not a design: `docs/adr/` holds the
binding decisions as they are made. Where a decision record says something,
this file cites it rather than restating it.

## Thesis

ABACUS is a software factory over `br` for work state and Herdr for agent
orchestration: a human-in-the-loop planning flow turns operator intent into a
well-defined backlog, and teams of provider-agnostic agents autonomously
clear it.

*Provider-agnostic* applies to the agents — Claude, Codex, and whatever comes
next are swappable. `br` and Herdr are pinned substrate, not seams the engine
abstracts over.

## Beneficiary

The operator, personally, running ABACUS as an execution engine across one or
more repositories at the same time. There is no second user.

Simultaneous multi-repo operation is a constraint, not a detail: it is what
puts shared substrate — one store, one Herdr server, one machine's worth of
panes — under real concurrency.

## Success condition

A backlog of N ready beads drains to closed overnight across two repositories
with **zero operator interventions**. Merge conflicts across the resulting
commits and PRs are identified and resolved by the engine. The final clean
PR(s) are ready for operator review in the morning. On repositories the
operator opts in, the engine also merges pending PRs during the overnight
run — serially, each validated against the main it actually lands on — and
the morning report becomes merged main plus any parked PRs carrying their
failure evidence (ADR 0003).

Autonomy ends at the PR, except on repositories opted into overnight
merging. Acceptance happens inside the agent team — a reviewer accepts, the
bead closes — while the human review gate sits at the merge boundary.

## Non-goals

Scoped to this thesis. Each may be revisited through revise mode — an
explicit operator act; none is admissible as "future-facing" work while it
stands here.

1. **Slack or any chat integration.** The morning's PRs are the report.
2. **Third-party distribution** — install flows, onboarding documentation,
   cross-version compatibility for anyone who is not the operator.
3. **Merging to main outside an opted-in overnight run, or any final
   acceptance authority beyond it.** The default still ends at "ready for
   operator review"; opting a repository in is an explicit operator act.

Permanent, not scoped:

4. **Being a general-purpose agent framework.** ABACUS orchestrates agents
   against a backlog. It is not a library for arbitrary agent workflows.

## Kill criteria

The thesis is wrong, and the work stops or pivots, when **the machinery makes
execution slower than simply running one or two vanilla coding-agent
sessions** on a comparable backlog.

The engine exists to buy throughput under autonomy. If the orchestration,
verification, and coordination cost more wall-clock than the naive baseline
delivers, no other property redeems it.

Explicitly *not* kill criteria: unattended runs that need intervention, and
review that yields too little leverage. Both are defects to debug, not
evidence the goal is wrong.

## Amendments

### 2026-08-14 — planning enters the thesis

- **Prior version:** blob `94b2ad5de415ee066de1b85442828c6ea5b77443`
  (founding text, commit `5a6ad81`), unchanged until this revision.
- **What changed:** the thesis was rewritten from execution-engine-only to a
  software factory whose product includes the human-in-the-loop planning
  flow; former non-goal 3 ("Planning or backlog authoring") was removed and
  the list renumbered; the introduction's document map was corrected to this
  repository's reality (dangling references to `CONTEXT.md` and
  `docs/migration.md` removed); the non-goals preamble was reworded to state
  how revision actually governs. The success condition and kill criteria are
  deliberately untouched: the planning flow's own success bar will be defined
  in its decision record and promoted here only with evidence.
- **Evidence and rationale:** the execution loop is demonstrated end to end
  — dispatch, outcome verification from bead state, lane reaping, and
  worker-opened PRs, across eight worker lanes on 2026-08-13/14 — while the
  backlog that feeds it still comes from a planning flow (`sable-plan`)
  built for substrate this machine has retired (the `bd` tracker, SABLE
  interlock hooks, and the manager fleet). The operator directs adapting
  that flow into the product: skill first, machinery only after observed
  need. Revision made on explicit operator invocation per revise mode.

### 2026-08-15 — overnight merging enters the goals for opted-in repositories

- **Prior version:** blob `3109a8ebea557aa38398c4b6f570f05538513c07`
  (as amended 2026-08-14, commit `679bbcc`).
- **What changed:** the success condition gains one sentence — on
  repositories the operator opts in, the engine merges pending PRs during
  the overnight run, serially, each validated against the main it lands
  on, with parked PRs as the morning exceptions; "Autonomy ends at the
  PR" gains the opted-in exception clause; non-goal 3 is qualified to
  exclude merging only outside an opted-in overnight run. Deliberately
  untouched: the thesis, beneficiary, non-goal 4, and kill criteria —
  the operator held the amendment to the minimum that licenses the
  decided work, and repo-agnostic design detail stays in ADR 0003.
- **Evidence and rationale:** the `ab-automerge-2b2` full planning run
  (FRAMING, RESEARCH, ARCHITECTURE, TEST-STRATEGY gates all
  operator-approved 2026-08-14/15), recorded in ADR 0003. Operator
  direction: an overnight multi-bead run leaves the base moving only when
  merges happen, and on lower-risk repositories merge throughput
  outranks morning review. Revision made on explicit operator invocation
  per revise mode.

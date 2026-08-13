# ABACUS — North Star

Established 2026-08-12 by operator interview (`/north-star`, establish mode).
Amendments happen only through revise mode, never as a consequence of an
inconvenient check.

This document states the goal. It is not a design: `CONTEXT.md` is the
normative product contract, `docs/adr/` holds the binding decisions, and
`docs/migration.md` holds the build plan. Where they say something, this file
cites them rather than restating it.

## Thesis

ABACUS is an execution engine over `br` for work state and Herdr for agent
orchestration, letting teams of provider-agnostic agents operate autonomously
as a software factory that clears a well-defined backlog of work.

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
PR(s) are ready for operator review in the morning.

Autonomy ends at the PR. Acceptance happens inside the agent team — a
reviewer accepts, the bead closes — while the human review gate sits at the
merge boundary.

## Non-goals

Scoped to this thesis. Each may be revisited through revise mode once the
success condition above is met; none is admissible as "future-facing" work
before then.

1. **Slack or any chat integration.** The morning's PRs are the report.
2. **Third-party distribution** — install flows, onboarding documentation,
   cross-version compatibility for anyone who is not the operator.
3. **Planning or backlog authoring.** The thesis consumes a *well-defined*
   backlog; producing one is upstream of the engine.
4. **Merging to main, or any final acceptance authority.** Excluded by the
   success condition, which ends at "ready for operator review."

Permanent, not scoped:

5. **Being a general-purpose agent framework.** ABACUS orchestrates agents
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

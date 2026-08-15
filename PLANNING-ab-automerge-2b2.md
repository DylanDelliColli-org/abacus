```doc-meta
role: planning-inflight
lifecycle: inflight
```

# PLANNING — ab-automerge-2b2 (PR validation and auto-merge machinery)

**Tier: FULL** — confirmed by operator 2026-08-14.

**Rationale:** the ask introduces new contracts (merge policy, validation
gate, risk-tier classification), carries real unknowns (moving-base
dynamics during a multi-bead run, validation semantics for an unattended
merge), amends the thesis (NORTH-STAR non-goal 3 and the success
condition currently end autonomy at the PR), and has a blast radius
spanning the engine, the worker protocol, at least one ADR, and the
north star. Every Quick criterion fails.

**Operator direction seeding the ask (2026-08-14):** without engine-side
validation and merging, the base moves constantly during an overnight
multi-bead run. Beyond PRs-waiting-in-the-morning, the operator wants an
autonomous mode for lower-risk or lower-complexity projects where
auto-merge throughput matters more than morning review.

Substages append below: FRAMING, RESEARCH, ARCHITECTURE, TEST-STRATEGY,
RECORD, DECOMPOSITION. This file is deleted at handoff; git history is
the archive.

---

## FRAMING

Produced live with the operator, 2026-08-14. Four load-bearing decisions
were put to the operator explicitly; all four answers are recorded under
"Operator decisions" below.

### User stories

- **S1 — opt-in mode.** As the operator, I mark a repository as
  auto-merge eligible, so an overnight run on it merges validated PRs
  without me. A repository without the mark behaves exactly as today.
- **S2 — validate against the landing base.** Every auto-merged PR is
  validated against the main it will actually land on: the engine brings
  the branch up to date with current main and requires the full
  validation suite green *after* that update. A PR that was green when
  the worker pushed, but is stale against moved main, can never land on
  its old evidence.
- **S3 — serialized merges.** The engine merges one PR at a time; each
  subsequent PR revalidates against the post-merge base. The moving base
  becomes the mechanism rather than the hazard.
- **S4 — park on failure.** A PR that fails validation is parked: left
  open with the failure evidence attached, never merged, never
  discarded. The run continues with other work. Parking is the safety
  net for every failure class, including a conflict resolution whose
  result does not validate green (S6).
- **S5 — default unchanged.** Without the opt-in, autonomy still ends at
  the PR and morning review is untouched.
- **S6 — engine-resolved conflicts, in the wedge.** When updating a PR
  onto moved main hits a merge conflict, the engine resolves the
  conflict itself (approach to be locked in ARCHITECTURE; candidate
  shapes include mechanical strategies and dispatching a resolution
  agent). The resolved result must then pass S2 validation before merge;
  a resolution that fails validation parks per S4.

### Non-goals

- CI/CD or deploy integration — validation is the local suite, not
  GitHub checks.
- Rollback automation for merged work.
- Per-bead risk scoring or complexity inference.
- Changes to the codex review seat or any operator-invoked review flow.
- A general merge-queue product (north-star non-goal 4 stands).

### Epic success metric

An overnight run on an auto-merge-enabled repository drains a backlog of
at least 3 beads to merged-to-main with zero operator interventions and
the main suite green in the morning.

### Narrowest valuable wedge

After a bead closes and its PR opens, the engine — in auto-merge mode
only — updates the branch onto current main, resolving a conflict itself
if one arises (S6), runs the full validation suite locally, merges on
green, and parks on red. Serialized, one PR at a time, gated by a
per-repository opt-in flag.

### Prerequisites

No existing bead is a prerequisite. One in-run prerequisite: the
NORTH-STAR revise-mode amendment (non-goal 3 carve-out plus a
success-condition variant) lands at RECORD, citing the ADR, before any
implementation child is authored at DECOMPOSITION. The amendment is an
operator act.

### Operator decisions (2026-08-14)

1. **North-star amendment timing: at RECORD.** The amendment cites the
   locked decision record instead of licensing a design that does not
   exist yet. Direction is decided now; the act happens at RECORD.
2. **Validation is mechanical only.** Branch updated onto current main,
   then full suite, clippy, and fmt green. No agent review leg in
   auto-merge mode; a review leg remains available as a later risk-tier
   knob if observed need arises.
3. **Risk tier is a per-repository flag.** The operator declares a
   repository auto-merge eligible in engine configuration or invocation.
   Finer granularity (per-bead labels) only after observed need,
   per the MVP-first ruling.
4. **Conflict resolution is in the wedge** (operator override of the
   planner's park-first recommendation). The success condition already
   promises engine-resolved conflicts; the operator wants the wedge to
   honor it directly rather than parking conflicts for morning.

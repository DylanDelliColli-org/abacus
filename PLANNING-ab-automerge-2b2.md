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

Produced live with the operator, 2026-08-14. Seven load-bearing
decisions were put to the operator explicitly (four in the first pass;
three more after the operator widened scope to include CI/CD mid-gate).
All answers are recorded under "Operator decisions" below.

### User stories

- **S1 — opt-in mode.** As the operator, I mark a repository as
  auto-merge eligible, so an overnight run on it merges validated PRs
  without me. A repository without the mark behaves exactly as today.
- **S2 — validate against the landing base.** Every auto-merged PR is
  validated against the main it will actually land on: the engine brings
  the branch up to date with current main and requires validation green
  *after* that update. Validation has two legs — the local leg (full
  suite, clippy, fmt on the updated branch) and the remote leg (GitHub
  CI checks green on the updated head, S7). A PR that was green when
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
- **S7 — CI is a co-validator.** The merge gate requires GitHub CI
  checks green on the updated head in addition to the local leg. The
  remote leg is canonical evidence that survives an operator-host crash
  mid-run; the local leg fails fast in seconds.
- **S8 — standard CI is a deliverable.** A standard workflow (test,
  clippy, fmt on PR and on main) ships in this epic, installed first on
  abacus itself. Auto-merge eligibility requires CI present on the
  repository.
- **S9 — repo-agnostic by design.** The merge queue works against any
  repository the operator runs it on; abacus is merely the first.
  Nothing in the machinery is special-cased to this repository.

### Non-goals

- Deploy-side machinery. Merge to main triggers whatever pipeline the
  repository already has; CD standardization is a future ask.
- Third-party distribution. Generality means repo-agnostic design, not
  install flows or external users; north-star non-goal 2 stands for
  this epic.
- Rollback automation for merged work.
- Per-bead risk scoring or complexity inference.
- Changes to the codex review seat or any operator-invoked review flow.

### Epic success metric

An overnight run on an auto-merge-enabled repository drains a backlog of
at least 3 beads to merged-to-main with zero operator interventions and
the main suite green in the morning.

### Narrowest valuable wedge

After a bead closes and its PR opens, the engine — in auto-merge mode
only — updates the branch onto current main, resolving a conflict itself
if one arises (S6), runs the local validation leg, waits for CI green on
the updated head (S7), merges on both-green, and parks on red.
Serialized, one PR at a time, gated by a per-repository opt-in flag on a
repository that has CI (S8).

### Prerequisites

No existing bead is a prerequisite. One in-run prerequisite: the
NORTH-STAR revise-mode amendment lands at RECORD, citing the ADR, before
any implementation child is authored at DECOMPOSITION. The amendment
covers non-goal 3 (merging to main — carved out for auto-merge mode), a
success-condition variant, and the merge-queue aspect of non-goal 4
(the queue is a general, repo-agnostic capability). The amendment is an
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
5. **CI joins the validation gate** (operator scope widening,
   mid-FRAMING). Both legs required: local suite after the update, and
   GitHub checks green on the updated head. Rationale: CI is becoming
   standard across the operator's repositories anyway, and remote
   evidence survives a host crash.
6. **Standard CI ships in this epic**, starting with abacus.
   Auto-merge eligibility requires CI present.
7. **CD stays out.** Merge triggers existing pipelines only; the engine
   builds nothing deploy-side.
8. **General capability, not rethesis** (operator scope widening,
   mid-FRAMING: "Abacus has reached the ceiling of its current North
   Star"). The merge queue is repo-agnostic by design; the beneficiary
   remains the operator. The RECORD amendment covers the merge-queue
   aspect of non-goal 4 within the current north star. A full rethesis,
   if pursued, is its own /north-star revise session later.
9. **No third-party distribution in this epic.** Non-goal 2 stands
   here regardless of any later rethesis.

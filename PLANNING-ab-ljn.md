# PLANNING — ab-ljn: tech-debt prevention reviewer

**Tier: FULL**, confirmed by the operator 2026-08-27.

Rationale for the tier: the ask introduces a new contract rather than a
change to an existing one. Every layer of the review path is currently keyed
to exactly one reviewer per bead per cycle — the agent name
(`reviewer_name`, `src/review.rs:418`, which doubles as the liveness signal
the sweep reads at `src/main.rs:1731`), the brief path (`brief_path`,
`src/review.rs:426`), the commit-status context (`STATUS_CONTEXT`, a single
string at `src/review.rs:11`), and cycle bookkeeping (heading counting
against a single `VERDICT_HEADING_PREFIX`, `src/review.rs:204-210`). A
second reviewer touches all of them. There are also genuine unknowns to lock
— chiefly what a split verdict means — and a blast radius spanning
`src/review.rs`, `src/lane.rs`, `src/main.rs`, `tests/drain.rs`, an ADR, and
the adjudication protocol in `.claude/skills/abacus-execute/SKILL.md`.

Operator decisions taken at tier selection, binding on everything below:

- **Advisory, not gating.** No second required commit status. A tech-debt
  finding can never hold a correct, working PR out of main.
- **Pre-existing findings go to `jot` only.** They enter the bead pool
  solely through operator-invoked `/jot-review`, preserving the Prime
  Directive.
- **Both reviewers run every cycle, in parallel.**

---

## FRAMING

### User stories

Stable identifiers for traceability through TEST-STRATEGY and DECOMPOSITION.

- **TD-1** — When a lane reaches `AwaitingReview`, a tech-debt reviewer
  launches in parallel with the correctness reviewer and posts its report as
  a PR comment.
- **TD-2** — The report states whether the PR is **minimal** for its bead's
  goal, naming specific code that could be removed or should not have been
  written.
- **TD-3** — The report evaluates architectural strategy, design principles,
  and data-structure choice against the patterns already in the repository,
  rather than against generic best practice.
- **TD-4** — Pre-existing architectural problems the reviewer notices while
  reading are captured to `jot` with enough detail to curate later
  (`--file`, `--symptom`, `--repro`), and are never minted as beads.
- **TD-5** — The reviewer almost never proposes work that increases line
  count. When it does, it must state explicitly why the increase is
  warranted; an unjustified net-addition proposal is a defect in the review.
- **TD-6** — The tech-debt report never blocks a merge, and the existing
  `adversarial-review` gate behaves exactly as it does today.
- **TD-7** — The engine does not confuse the two reviewers: a tech-debt
  report is never counted as a correctness verdict, never satisfies a
  correctness cycle, and never suppresses or triggers a correctness reviewer
  relaunch.
- **TD-8** — The operator can ignore a tech-debt report entirely without the
  lane wedging, the drain looping, or reviewers relaunching.

### Non-goals

1. **No second required status and no merge gate.** Operator decision above.
   Promoting the reviewer to gating later is a separate, explicit act.
2. **Not a replacement for the correctness reviewer.** The two run
   independently and neither subsumes the other.
3. **The reviewer does not rework.** It reports; it never edits code. The
   existing one-permitted-write ground rule applies unchanged, extended only
   to permit its `jot` captures for TD-4.
4. **No beads for pre-existing debt.** Operator decision above.
5. **Not a general linter or style checker.** This is a per-PR architectural
   review, not a rule engine. Anything mechanically checkable belongs in
   clippy or CI, not here.
6. **No new agent provider.** Codex-first, matching the existing ephemeral
   reviewer (ADR 0005 D3).
7. **Not retroactive.** It reviews PRs in flight; it does not sweep the
   existing codebase for debt.

### Epic success metric

**Proposed, pending operator decision — see Open question OQ-2.**

Candidate: across the first 20 PRs reviewed by both agents, at least 50%
carry one or more tech-debt findings the operator accepts.

The metric is deliberately about *accepted* findings rather than findings
produced, because a reviewer that reliably produces ignored output has
failed regardless of volume. It is also deliberately not a line-count trend:
diff size is confounded by bead scope, so a falling median would not
attribute to this reviewer.

### Narrowest valuable wedge

Ship the parallel reviewer against the abacus repository itself, advisory,
with:

- its own deterministic agent name, brief path, and report heading, all
  distinct from the correctness reviewer's;
- a report posted as a PR comment;
- `jot` capture for pre-existing findings (TD-4); and
- **strict isolation from correctness bookkeeping** (TD-7) — the engine's
  cycle counting, relaunch decision, reaping, and status reconciliation must
  behave identically whether or not a tech-debt report exists.

Explicitly outside the wedge: any second commit status, any adjudication
requirement for the tech-debt report, any engine parsing of the report body,
and any cross-repo rollout.

TD-7 is the wedge's load-bearing property. Everything else is additive; a
failure of TD-7 corrupts the correctness gate that already works.

### Prerequisites

Four open beads touch the exact surfaces this epic modifies. Ordering is an
open question (OQ-1); the interactions are facts.

- **`ab-cye`** — *verdict heading must be the first body line, so a relayed
  verdict with an attribution preface is invisible and its cycle is
  re-reviewed.* Directly load-bearing for TD-7: this epic introduces a
  **second heading grammar on the same PR**, and `ab-cye` decides whether
  heading detection becomes tolerant (scan for the heading) or stays strict
  (first line only). A tolerant scan makes miscounting the tech-debt report
  as a correctness verdict substantially easier to get wrong.
- **`ab-xuz`** — *amend the canonical adversarial-review contract with nine
  field-proven amendments.* Rewrites `REFUTATION_BRIEF_TEMPLATE`. The
  tech-debt reviewer needs its own brief that shares the ground rules
  (read-only posture, the one-permitted-write rule, the evidence bar, the
  Probes requirement). If this epic lands first, `ab-xuz` must amend two
  templates instead of one.
- **`ab-5lw`** — *add the operator's verdict-neutrality clause to
  `REFUTATION_BRIEF_TEMPLATE`.* Same region as `ab-xuz`, same argument. Its
  neutrality principle — let the executed evidence decide, do not steer
  toward a verdict — applies with equal force to a simplification reviewer,
  which has an obvious bias toward finding something to cut.
- **`ab-645`** — *`sanitize_agent_name` 32-char truncation collides for deep
  child bead ids and wedges drain.* `reviewer_name` computes
  `capacity = 32 - ("rev-" + "-c<n>").len()`. A second reviewer needs a
  distinct prefix, which changes that arithmetic and adds a second name
  collision surface — the exact class `ab-645` fixes. Doubling the reviewer
  population per cycle also doubles the exposure.

---

## Open questions

- **OQ-1 — Prerequisite ordering.** Does this epic block on `ab-cye`,
  `ab-xuz`, `ab-5lw`, and `ab-645`, or does it proceed against current HEAD
  with declared ordering and accept the rework? Unanswered.
- **OQ-2 — Epic success metric.** Is the proposed accepted-findings metric
  the right one, and is 50% of 20 PRs the right bar? Unanswered. Related
  tension: the north star's kill criterion is that the machinery must not
  make execution slower than vanilla agent sessions, and this reviewer
  doubles review cost on arcs documented at up to 21 cycles.
- **OQ-3 — Verdict grammar for an advisory reviewer.** Does the tech-debt
  report carry a REFUTED / NOT REFUTED verdict line at all, given that
  nothing acts on it? Deferred to ARCHITECTURE; recorded here so it is not
  lost.

*Status: FRAMING drafted, awaiting operator gate. RESEARCH not started.*

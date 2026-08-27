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
- **Two separate agents, not one agent with a wider brief.** Confirmed at
  the FRAMING gate after the operator raised it directly. The deciding
  argument is a recurring posture conflict: a correctness finding very often
  *demands more code* — add a guard, validate an input, handle a case —
  while TD-5 says almost never propose a line-count increase. A single
  context holding both mandates must arbitrate that silently, per finding,
  invisibly to the operator. Two reports make the tradeoff explicit and
  adjudicable. Supporting: `ab-5lw` exists because brief framing biases
  reviewer incentives hard, and an agent with two mandates has two
  independent incentives to manufacture findings; ADR 0005 D3 makes
  reviewers deliberately fresh and ephemeral per cycle, so a second
  discipline in the same context halves the attention each receives — on top
  of the nine amendments `ab-xuz` already adds to that brief.

  Recorded because it was close: folding would have eliminated TD-7
  entirely, dropped the `ab-cye` and `ab-645` dependencies, roughly quartered
  the work, and halved the review cost this feature adds. It was rejected on
  the posture conflict, not on cost.

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

**Decided at the FRAMING gate 2026-08-27:** across the first 20 PRs reviewed
by both agents, at least 50% carry one or more tech-debt findings the
operator accepts.

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

**Decided at the FRAMING gate 2026-08-27: this epic blocks on all four.**
Wired as `br` dependencies on `ab-ljn`; `br blocked` confirms
`ab-ljn` blocked by `ab-5lw`, `ab-645`, `ab-cye`, `ab-xuz`. The reviewer
contract therefore reaches its final form before a second brief derives from
it, and no rework is needed at the seams.

These gate **implementation, not planning.** This planning run proceeds now;
the epic simply does not appear in `br ready` until the four close.
DECOMPOSITION must decide whether each implementation child carries the
dependencies itself or inherits the block from the epic — an epic-level
block alone does not stop a child from appearing ready.

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

- **OQ-1 — Prerequisite ordering.** RESOLVED 2026-08-27: block on all four
  (`ab-cye`, `ab-xuz`, `ab-5lw`, `ab-645`), wired as `br` dependencies.
- **OQ-2 — Epic success metric.** RESOLVED 2026-08-27: accepted-findings
  rate, 50% of the first 20 PRs. The cost tension against the north star's
  kill criterion is noted but not made part of the metric; if the second
  reviewer proves to slow drains materially, that is a kill-criterion
  conversation, not a metric adjustment.
- **OQ-3 — Verdict grammar for an advisory reviewer.** Does the tech-debt
  report carry a REFUTED / NOT REFUTED verdict line at all, given that
  nothing acts on it? Deferred to ARCHITECTURE by design; recorded here so
  it is not lost. Note the interaction with TD-7: reusing the correctness
  verdict vocabulary on the same PR is the most likely route to a
  miscounted cycle.
- **OQ-4 — Separate agent or wider brief.** RESOLVED 2026-08-27 at the
  FRAMING gate: two separate agents. Rationale recorded under the operator
  decisions at the top of this file.
- **OQ-6 — Is "tech-debt prevention" one agent or several?** Raised
  2026-08-27 by operator-supplied prior art. NousResearch hermes-agent
  issue 379 proposes *three* parallel reviewers — code reuse, code quality,
  efficiency — on the explicit ground that "three focused agents > one
  general agent" because each can search deeply "without context dilution."
  That is the same argument that decided OQ-4 one level up. This epic
  currently bundles three concerns into one agent: minimality, architectural
  strategy, and data-structure choice. **Planner recommendation: keep one
  agent.** Unlike correctness-versus-minimality, these three share a single
  lens — "is this the right shape for the problem?" — and do not pull
  against each other, so the dilution argument is much weaker. Splitting
  further also multiplies review cost against a kill criterion this epic
  already strains. Operator decision required at the RESEARCH gate.
- **OQ-5 — Does TD-5 as written encode a known anti-pattern?** Raised
  2026-08-27 by operator-supplied prior art, after FRAMING was approved.
  **Now supported by three independent sources, not one** — see the RESEARCH
  inputs section. Planner recommendation is firm: reword TD-5 to target
  *unnecessary code and unnecessary concepts* rather than *line count*, and
  state explicitly that a change which adds lines while removing a concept
  is a valid simplification. Operator decision required at the RESEARCH gate.
  Anthropic's official `code-simplifier` agent
  (`~/.claude/plugins/marketplaces/claude-plugins-official/plugins/code-simplifier/agents/code-simplifier.md`,
  section 4 "Maintain Balance") explicitly warns against prioritising
  "fewer lines" over readability — naming nested ternaries and dense
  one-liners — and instructs choosing clarity over brevity. TD-5 currently
  says the reviewer "almost never proposes work that increases line count."
  These are in tension. The question is whether TD-5 should be reworded to
  target *unnecessary code* rather than *line count*, which would preserve
  the operator's intent while avoiding the documented failure mode.
  Deferred to the RESEARCH gate, where a concrete recommendation is due.
  **TD-5 stands as approved until the operator amends it.**

## RESEARCH inputs supplied by the operator

**Convergent finding across the set: every source that takes a position
rejects line count as the objective.** Anthropic's simplifier lists
prioritising "fewer lines" over readability as an over-simplification
failure. The `agentic-awesome-skills` skill states "the goal is not fewer
lines" and rejects "fewer lines is always simpler" as a named
rationalisation, adding: "a 1-line nested ternary is not simpler than a
5-line if/else. Simplicity is about comprehension speed, not line count."
The `githubnext` workflow states "explicit code is often better than compact
code." Three independent authors, same conclusion. This is the evidence
behind OQ-5.

- Anthropic's official `code-simplifier` agent (path above). A
  byte-identical body also ships in the same marketplace's
  `pr-review-toolkit` plugin, differing only in frontmatter examples.
  Assessed as prior art for **posture and guardrails only**: it is an
  *editing* agent that applies changes autonomously, carries no evidence
  bar, threat model, severity grading, verdict grammar, or probes
  requirement, and its standards section is JS/TS/React-specific. It is not
  a template for a read-only adversarial reviewer.
- **`agentic-awesome-skills` code-simplification skill**
  (github.com/sickn33/agentic-awesome-skills). An editing skill, but its
  rules are the sharpest of the set. Directly reusable for our brief:
  *"Simplification requiring modified tests"* is listed as a **red flag**
  meaning behaviour probably changed — a precise, checkable invariant for a
  reviewer proposing a simplification. Also: never simplify code you do not
  understand; leave no dead code; do not weaken error handling; do not
  rename by preference rather than convention. Its "when NOT to use" list
  (code already clean; module about to be rewritten; performance-critical
  path) is a usable false-positive guard.
- **`githubnext/agentics` code-simplifier workflow.** A scheduled editing
  workflow that opens PRs. Structurally unlike ours, but confirms the same
  posture: never change what the code does, run the tests before proposing,
  revert if they fail, prefer focused edits over rewrites.
- **`githubnext/agentics` duplicate-code-detector workflow.** *The closest
  structural analogue to this epic* — read-only, reports only, never
  modifies files. Three ideas worth taking:
  (a) an explicit **significance threshold** before reporting at all
  (">10 lines duplicated OR 3+ instances");
  (b) a hard **findings cap** — "limit to the top 3 most significant
  patterns" — which **collides with `ab-xuz` amendment 1**, requiring
  exhaustive sweep on stable designs; ARCHITECTURE must reconcile a volume
  cap against exhaustive enumeration;
  (c) an explicit **exclusion list** (tests, generated code, vendored deps,
  boilerplate, snippets under 5 lines, language idioms) as a false-positive
  guard. This epic needs its own.
- **NousResearch hermes-agent issue 379** — "Simplify Skill: Parallel Code
  Review & Cleanup". Independently corroborates the OQ-4 decision: it
  proposes parallel specialised reviewers on the explicit ground that
  "three focused agents > one general agent" because each searches deeply
  "without context dilution." Also the source of OQ-6.
- The same `pr-review-toolkit` plugin ships five sibling agents —
  `code-reviewer`, `comment-analyzer`, `pr-test-analyzer`,
  `silent-failure-hunter`, and `type-design-analyzer`. The last is closer to
  half of this epic's mandate than the simplifier is: it reviews types
  introduced by a PR and rates encapsulation, invariant expression,
  usefulness, and enforcement — the "optimal data structures" half of the
  ask. RESEARCH is tasked with assessing all of them.

*Status: FRAMING approved 2026-08-27. RESEARCH next.*

# PLANNING — ab-ljn: the reviewer stack

**Tier: FULL**, confirmed by the operator 2026-08-27.

**Scope expanded 2026-08-27, after FRAMING was first approved.** This epic
began as "add one advisory tech-debt reviewer." The operator widened it to
the reviewer stack as a whole, including whether the correctness reviewer
should become two faster, cheaper, more targeted models. The FRAMING below
is the rewrite; it supersedes the narrow version, which survives in git
history at commit `c75729e`.

The expansion is justified by a single fact discovered while grounding it:
**the engine has no model or agent-kind selection at all.** Both agent start
sites hardcode `--kind codex` and pass no agent arguments — reviewers at
`src/review.rs:543-553`, workers at `src/lane.rs:217-227` — while
`herdr agent start` supports 21 agent kinds and a `-- [AGENT_ARG]...`
passthrough that would carry a model flag. Nothing in abacus can express
"this reviewer runs a different model than that one."

Both changes the operator wants need that same missing seam, plus an engine
that tracks N reviewers per cycle rather than one. Planning the narrow epic
first would hardcode a second reviewer and design its isolation for exactly
two, then refactor all of it when correctness splits. **Breaking the
one-reviewer-per-cycle assumption once, generically, is the reason to expand
now.**

---

## Operator decisions binding on everything below

Taken at tier selection and at the two FRAMING gates. Each is recorded with
its rationale so a later session does not relitigate it.

1. **FULL tier.** Six gated substages.
2. **The tech-debt reviewer is ADVISORY.** No commit status of its own. It
   can never hold a correct, working PR out of main.
3. **Pre-existing findings go to `jot` only**, entering the bead pool solely
   through operator-invoked `/jot-review`. Preserves the Prime Directive: an
   automated reviewer minting beads on every PR is the landfill that rule
   exists to prevent.
4. **Every reviewer runs every cycle, in parallel.**
5. **Correctness and tech-debt are separate agents, not one wider brief.**
   Decided on a recurring posture conflict: a correctness finding routinely
   *demands more code* — add a guard, validate an input, handle a case —
   while the minimality mandate pushes the other way. One context holding
   both must arbitrate that silently, per finding, invisibly to the
   operator. Two reports make the tradeoff explicit and adjudicable.
   Independently corroborated by NousResearch hermes-agent issue 379
   ("three focused agents > one general agent... without context
   dilution"). Recorded because it was close: folding would have eliminated
   the isolation problem entirely and halved the added review cost. It was
   rejected on the posture conflict, not on cost.
6. **The correctness split is an experiment the seam enables, not a decision
   this run makes.** The claim "two fast targeted models beat one strong
   general model" is empirical, and no measurement exists — not for where
   review wall-clock goes, not for per-cycle cost, not for what a cheaper
   model would miss. The standing MVP-first ruling forbids machinery for
   unobserved conditions. ARCHITECTURE makes reviewer kind, model, and count
   configurable; the stack ships with correctness unchanged; splitting
   becomes cheap to try and is decided on data.
7. **No new instrumentation machinery in this epic.** Follows from 6: the
   operator declined the instrument-first option. Measurement for the split
   experiment comes from operator observation of real drains, not from new
   engine telemetry. *Stated risk:* if observation proves too coarse to
   decide the split, instrumentation becomes its own bead rather than
   silently expanding this one.

---

## FRAMING

### User stories

Two groups. `RS-*` are the stack itself. `TD-*` are the tech-debt reviewer,
the stack's first new consumer and the proof that the seam works.

**The stack**

- **RS-1** — Reviewer kind, model, and count are a declared seam rather than
  hardcoded values. The engine can launch N reviewers for a cycle, each with
  its own agent kind and model.
- **RS-2** — Engine bookkeeping is correct for N reviewers: cycle counting,
  relaunch decisions, reaping, and commit-status reconciliation all behave
  correctly when more than one reviewer reports on the same PR. This
  generalises what the narrow framing called TD-7 and remains the epic's
  load-bearing safety property.
- **RS-3** — Adding a reviewer to the stack is a configuration act plus a
  brief. It requires no change to engine control flow.
- **RS-4** — Gating authority is per-reviewer. A reviewer declares whether
  it owns a commit status and can block a merge, or is advisory. Correctness
  gates; tech-debt does not.
- **RS-5** — The correctness reviewer's observable behaviour is unchanged by
  this refactor: same brief, same model, same gate, same verdict grammar,
  same cycle semantics. This is the regression property.
- **RS-6** — Reconfiguring correctness from one reviewer into two targeted
  reviewers is a configuration change plus briefs, requiring no engine
  change. The experiment is cheap by construction.

**The tech-debt reviewer**

- **TD-1** — When a lane reaches `AwaitingReview`, the tech-debt reviewer
  launches alongside the correctness reviewer and posts its report as a PR
  comment.
- **TD-2** — The report states whether the PR is **minimal** for its bead's
  goal, naming specific code that should not have been written.
- **TD-3** — The report evaluates architectural strategy, design principles,
  and data-structure choice against the patterns already in this
  repository, not against generic best practice.
- **TD-4** — Pre-existing architectural problems the reviewer notices are
  captured to `jot` with enough detail to curate later (`--file`,
  `--symptom`, `--repro`), and never minted as beads.
- **TD-5** — *Wording under revision; see OQ-5.* The reviewer targets
  unnecessary code and unnecessary concepts. It does not treat line count as
  the objective.
- **TD-6** — The tech-debt report never blocks a merge.
- **TD-8** — The operator can ignore a tech-debt report entirely without the
  lane wedging, the drain looping, or reviewers relaunching.

`TD-7` is retired as a story identifier; its content is now `RS-2`.
References to TD-7 in commits before `c3d7ff8` mean `RS-2`.

### Non-goals

1. **No new instrumentation or telemetry.** Operator decision 7.
2. **The correctness split is not decided here.** Operator decision 6. This
   epic makes it cheap to try, nothing more.
3. **The reviewer never edits code.** The one-permitted-write ground rule
   stands, extended only to permit `jot` capture for TD-4.
4. **No beads for pre-existing debt.** Operator decision 3.
5. **Not a linter.** Anything mechanically checkable belongs in clippy or
   CI, not in an agent that costs a context per cycle.
6. **Not retroactive.** Reviewers judge PRs in flight; they do not sweep the
   existing codebase.
7. **No cross-repo rollout in this epic.** Prove the stack in abacus first.
8. **The worker/author path is out of scope.** `src/lane.rs:217-227`
   hardcodes `--kind codex` for workers too. Tempting to fix in passing;
   explicitly excluded to keep the blast radius on the review layer.

### Epic success metric

**Proposed at the re-gate, replacing the narrow epic's metric — see OQ-7.**

Candidate: **a third reviewer can be added to the stack with no change to
engine control flow** — configuration and a brief only, demonstrated by
adding one in a test.

Rationale for changing it: the previous metric (at least 50% of the first 20
PRs carry an accepted tech-debt finding) measures whether the *tech-debt
reviewer* earns its cost. That is still the right measure for that reviewer,
but it belongs on the tech-debt child, not on a stack epic. The stack's own
claim is architectural, and RS-3/RS-6 are only real if adding or
reconfiguring a reviewer costs no engine work.

### Narrowest valuable wedge

The reviewer-kind seam, N-reviewer bookkeeping, and the tech-debt reviewer
as its first consumer — in the abacus repository only, with correctness
behaviour held constant.

Explicitly outside the wedge: splitting correctness, any second gating
status, cross-repo rollout, worker-path model selection, and instrumentation.

`RS-2` and `RS-5` together are the load-bearing pair. Everything else is
additive; a failure of either corrupts a correctness gate that currently
works. The single worst outcome of this epic is an engine that believes a
correctness review happened when it did not.

### Prerequisites

**Unchanged by the expansion: this epic blocks on all four.** Wired as `br`
dependencies; `br blocked` confirms `ab-ljn` blocked by `ab-5lw`, `ab-645`,
`ab-cye`, `ab-xuz`.

- **`ab-cye`** — *verdict heading must be the first body line.* Decides
  whether heading detection becomes tolerant or stays strict, which directly
  determines how RS-2 isolation is implemented once several reviewers post
  headings to the same PR.
- **`ab-xuz`** — *nine amendments to the canonical reviewer contract.*
  Rewrites `REFUTATION_BRIEF_TEMPLATE`, which every reviewer in the stack
  derives shared ground rules from.
- **`ab-5lw`** — *verdict-neutrality clause.* Same region. Its principle —
  let executed evidence decide, do not steer toward a verdict — applies with
  extra force to a simplification reviewer, which is biased toward finding
  something to cut.
- **`ab-645`** — *`sanitize_agent_name` 32-char truncation collides.*
  `reviewer_name` computes `capacity = 32 - ("rev-" + "-c<n>").len()`. N
  reviewers per cycle need distinct prefixes, changing that arithmetic and
  multiplying the collision surface rather than merely doubling it.

These gate **implementation, not planning.** DECOMPOSITION must decide
whether each child carries the dependencies itself; an epic-level block does
not stop a child from appearing in `br ready`.

---

## Open questions

- **OQ-1 — Prerequisite ordering.** RESOLVED: block on all four.
- **OQ-2 — Epic success metric.** SUPERSEDED by OQ-7 under the expanded
  scope. The accepted-findings measure (≥50% of the first 20 PRs) is
  retained as the tech-debt reviewer child's own acceptance measure.
- **OQ-3 — Verdict grammar for an advisory reviewer.** Does an advisory
  report carry a verdict line at all, when nothing acts on it? Deferred to
  ARCHITECTURE by design. Interacts with RS-2: reusing the correctness
  verdict vocabulary on the same PR is the most likely route to a miscounted
  cycle.
- **OQ-4 — Separate agents or one wider brief.** RESOLVED: separate.
- **OQ-5 — Does TD-5 encode a known anti-pattern?** Effectively resolved by
  evidence; needs operator ratification of the wording. Three independent
  sources reject line count as the objective (see RESEARCH inputs).
  Recommended wording: target *unnecessary code and unnecessary concepts*,
  and state explicitly that a change adding lines while removing a concept
  is a valid simplification.
- **OQ-6 — Is tech-debt prevention one agent or several?** Open.
  hermes-agent 379 argues three focused agents beat one general agent on
  context dilution — the same argument that decided OQ-4 one level up. This
  epic bundles minimality, architectural strategy, and data-structure choice
  into one agent. **Recommendation: keep one.** These three share a single
  lens ("is this the right shape?") and do not conflict, unlike
  correctness-versus-minimality; splitting multiplies cost against a kill
  criterion this epic already strains. Note the expanded scope makes this
  cheaper to revisit later, since RS-3 makes adding a reviewer configuration.
- **OQ-7 — Epic success metric under the expanded scope.** Open. Is the
  proposed architectural metric (a third reviewer added with no engine
  change) the right epic measure?

---

## RESEARCH inputs supplied by the operator

**Convergent finding: every source that takes a position rejects line count
as the objective.** Anthropic's simplifier lists prioritising "fewer lines"
over readability as an over-simplification failure. The
`agentic-awesome-skills` skill states "the goal is not fewer lines" and
rejects "fewer lines is always simpler" as a named rationalisation, adding:
"a 1-line nested ternary is not simpler than a 5-line if/else. Simplicity is
about comprehension speed, not line count." The `githubnext` workflow states
"explicit code is often better than compact code." Three independent
authors, same conclusion. This is the evidence behind OQ-5.

- **Anthropic `code-simplifier`**
  (`~/.claude/plugins/marketplaces/claude-plugins-official/plugins/code-simplifier/agents/code-simplifier.md`).
  A byte-identical body also ships in the same marketplace's
  `pr-review-toolkit`. Prior art for **posture and guardrails only**: it is
  an *editing* agent applying changes autonomously, with no evidence bar,
  threat model, severity grading, verdict grammar, or probes requirement,
  and JS/TS/React-specific standards. Not a contract template for a
  read-only adversarial reviewer.
- **`agentic-awesome-skills` code-simplification skill.** An editing skill,
  but the sharpest rules of the set. Directly reusable: *"simplification
  requiring modified tests"* is a **red flag** meaning behaviour probably
  changed — a precise, checkable invariant. Also: never simplify code you do
  not understand; leave no dead code; do not weaken error handling; do not
  rename by preference over convention. Its "when NOT to use" list
  (already-clean code, module about to be rewritten, performance-critical
  paths) is a usable false-positive guard.
- **`githubnext/agentics` code-simplifier workflow.** Scheduled editing
  workflow that opens PRs. Confirms posture only: never change behaviour,
  run tests before proposing, revert on failure, focused edits over
  rewrites.
- **`githubnext/agentics` duplicate-code-detector workflow.** *The closest
  structural analogue* — read-only, reports only, never modifies files.
  Three ideas to evaluate: (a) an explicit **significance threshold** before
  reporting at all (">10 lines duplicated OR 3+ instances"); (b) a hard
  **findings cap** ("top 3 most significant patterns"), which **collides
  with `ab-xuz` amendment 1** requiring exhaustive sweep on stable designs —
  ARCHITECTURE must reconcile, and the likely resolution is the gating
  versus advisory asymmetry; (c) an explicit **exclusion list** (tests,
  generated code, vendored deps, boilerplate, sub-5-line snippets, language
  idioms).
- **NousResearch hermes-agent issue 379.** Corroborates decision 5 and is
  the source of OQ-6.
- **`pr-review-toolkit` siblings** — `code-reviewer`, `comment-analyzer`,
  `pr-test-analyzer`, `silent-failure-hunter`, `type-design-analyzer`. The
  last is closer to the data-structure half of TD-3 than the simplifier is:
  it reviews types introduced by a PR and rates encapsulation, invariant
  expression, usefulness, and enforcement. Under the expanded scope these
  are also **candidate stack members**, not just reference material.

*Status: FRAMING rewritten for the expanded scope, awaiting re-gate.
RESEARCH in flight against the original brief plus two extensions.*

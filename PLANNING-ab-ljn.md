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

---

## RESEARCH

Produced by a sherlock-type subagent, 2026-08-27, audited at `c75729e`. No
beads written, no source edited. Two incidental defects captured to `jot`.
Three seam questions (the config/model seam, per-reviewer gating in the
status code, and stack membership) arrived after the audit was largely
complete and are outstanding; they are ARCHITECTURE-shaped and do not block
this gate.

### FRAMING verdict: survives, with one correction

Nothing in FRAMING is invalidated. One operator decision is not achievable
as written, and one blast-radius omission was found.

**Operator decision 4 — "every reviewer runs every cycle, in parallel" — is
not achievable on the current launch path.** This is proven from code, not
inferred. `launch_reviewer` (`src/review.rs:557-562`) calls `prompt_agent`
(`src/lane.rs:361`) which calls `capture` (`src/lane.rs:643-660`) which runs
`Command::output()` — blocking until the child exits. With
`herdr agent prompt --wait`, herdr holds until the agent's turn ends, so
`launch_awaiting_reviewer` returns only after the review is *finished and
its verdict already posted*. Two launches placed in sequence cost
`t1 + t2`, not `max(t1, t2)`.

ADR 0005 D2 (`docs/adr/0005-lane-lifecycle-v2.md:104-114`) states this as
design: "At most one worker turn is active at any moment (the engine blocks
on `prompt --wait`); concurrency exists only as settled lanes awaiting
adjudication." Two live reviewer turns is not covered by that carve-out.
**Genuine parallelism is a D2 amendment, not merely a code change.**

**Blast-radius omission: `src/lane.rs`.** FRAMING did not name it. The
non-blocking launch seam lives at `src/lane.rs:340-456`, and it is the
epic's highest-risk module — `prompt_agent`'s zero-effect-settle recovery
(baseline context sample → prompt → post-settle sample → conditional Enter
nudge → wait-working → wait-done) is sequential by construction and cannot
be interleaved without restructuring. RESEARCH rated its own confidence
MEDIUM here, the only module below HIGH.

### S1 — the critical isolation failure, and it is reachable

`heading_cycle` (`src/review.rs:97-101`) does `strip_prefix` then
`take_while(is_ascii_digit)`. **It is a prefix match and ignores trailing
text**, so `## Adversarial review — cycle 3 — tech debt` parses as cycle 3.
Any tech-debt heading beginning with the correctness prefix registers a
phantom cycle. Then, within one `reconcile_review_lifecycle` call, in
execution order:

1. `reap_reviewers_with_verdicts` (`src/main.rs:1678-1696`) closes
   `rev-<bead>-c<n>`'s workspace **with no `agent_status` guard** — unlike
   its sibling at `src/main.rs:1737`. A correctness reviewer mid-review is
   killed.
2. `src/main.rs:1728-1730` then sees that cycle already in `verdict_cycles`
   and returns early. **It never relaunches.**

How it presents: the lane sits `awaiting-review`, status pending, no
reviewer alive, no verdict ever posted, and every subsequent drain exits 0
printing `awaiting-review: 1`. `.claude/skills/abacus-execute/SKILL.md:65`
tells the orchestrator that `AwaitingReview` means a verdict is *owed*, not
that anything stalled — **the documentation steers away from the correct
diagnosis.**

Worse, the same root reaches the merge gate. With cycle N now "reviewed",
`latest_reviewed_adjudication` (`src/main.rs:1625-1630`) accepts an
adjudication for N. An operator adjudicating the tech-debt report — which
`SKILL.md:46` actively invites — flips `adversarial-review` to `success` at
`src/main.rs:1655-1666` and, on a repo where that check is required,
**clears a PR to merge with zero correctness review performed.** This is
exactly the outcome FRAMING named as the worst possible, and it is two
ordinary steps away.

Mitigation is trivial but must be a **hard constraint, not a convention**:
no other reviewer's heading may begin with `## Adversarial review — cycle `.

**S2, conditional on `ab-cye`:** if `ab-cye` resolves toward a tolerant
scan, a tech-debt report that merely *quotes* the correctness heading
triggers the whole S1 chain. This RESEARCH must feed `ab-cye` — whatever
tolerance it adopts must be line-anchored, and reports must be forbidden
from quoting the heading.

**S4, HIGH:** both reapers match only `reviewer_name(bead_id, cycle)`
(`src/main.rs:1684`, `:1731`). An agent under any other name is invisible to
both, so **tech-debt workspaces are never reaped.** At the 22-cycle depth
recorded in `ab-xuz`, that is 22 orphan workspaces on one PR. A second
reaper is mandatory.

**S8, pre-existing and doubled:** `cmd_run` (`src/main.rs:751`) and
`cmd_drain`'s settle arm (`src/main.rs:887`) pass an *empty* agent slice
into `reconcile_review_lifecycle`, so every liveness lookup and reap on
those paths is already a no-op. Only `sweep_live_lanes` supplies real
agents. The dispatch path can already launch a reviewer whose predecessor
still runs; a second reviewer doubles the exposure.

### OQ-3 is answered on the engine side

**The engine never reads a verdict body.** `VERDICT_REFUTED`,
`VERDICT_NOT_REFUTED`, and `PROBES_HEADING` (`src/review.rs:17-19`) appear
only in brief-template substitution at `src/review.rs:438-440`; a grep
across `src/main.rs`, `src/lane.rs`, `src/lib.rs`, and `src/land.rs` returns
zero non-test reads. Reusing the REFUTED / NOT REFUTED vocabulary in an
advisory report is therefore **engine-inert**. The entire isolation risk
lives in the *heading* and in operator confusion — not in the verdict line.

### Prior art: this repo already runs a two-reviewer stack

**Every ADR in this repository was reviewed by two independent
fresh-context reviewers with different mandates** — a *bloat review* (cut
unnecessary scope) and a *spec validation* (faithfulness), with operator
dispositions recorded per finding. Trail at
`docs/adr/0001-planning-flow.md:9-15` and `:232-239`,
`0002:9-10`, `0003:9-14`, `0004:11`, `0005:12-35`. Live artifacts remain at
`PLANNING-adr4-bloat-review.md`, `PLANNING-adr4-cut-positions.md`,
`PLANNING-adr4-spec-review.md`.

This is the repo's own proven idiom for exactly the shape this epic
proposes, in a different domain. **Directly usable for ARCHITECTURE:** the
bloat review's per-cut output form — each cut carrying a **"Cost of
cutting"** and a **"Revive when"** clause (recoverable at
`git show b256cb8^:PLANNING-adr5-bloat-review.md`) — is a field-proven,
operator-disposable shape for TD-2 and TD-5 findings, and it is native
rather than imported.

Also native: `AGENTS.md:112-115` already requires read-only review
dispatches to state that the review is not bead-tracked work — "no beads,
no branches, no commits" — because "a reviewer that follows the prime
directive without this line leaves tracker and remote exhaust." **TD-4 is an
extension of an existing field-proven rule, not a new invention.**

**No rule forbids this epic.** There is no "exactly one reviewer" clause in
any ADR, doc, bead, or commit. The nearest fence is ADR 0005's bloat-review
cut-1 disposition (`:12-21`), noting that moving check-flip authority to an
agent reviewer would need a fresh operator ruling — which the advisory-only
decision sidesteps entirely.

**Explicit negative:** no line-count or diff-size discipline exists anywhere
in the repo — no `AGENTS.md` rule, no `CONSTRAINTS.md` finding, no ADR
clause, no CI check. TD-5 would be the first. Zero hits for "minimality",
"tech debt", "second reviewer", or "parallel reviewer" outside this planning
file.

### The 37 single-reviewer assumptions

RESEARCH enumerated every site assuming one reviewer per (bead, cycle): 17
in code, 5 in contracts, and 15 in tests. Full list in the report; the
load-bearing ones are `reviewer_name` and `brief_path` as pure functions of
(bead, cycle) (`src/review.rs:418-429`), `verdict_cycles` as a **deduped
set** that cannot express partial completion (`src/review.rs:236-237`), the
`launched_reviewers` key `(bead_id, cycle)` (`src/main.rs:1735`), and both
reapers.

One useful discovery: **the quorum seam already exists and is unused.**
`verdict_heading_count` is computed at `src/main.rs:1796` and `:1939`,
carried through `LaneStateInputs` (`src/lane.rs:89`), and then discarded at
`src/lane.rs:121` with `let _ =`. RESEARCH recommends **not** using it — an
advisory reviewer that gates lane state contradicts TD-6 — but it is there.

### The strongest decomposition constraint

**The test-harness generalization must land in the same bead as the first
launch change, or the suite is red between beads.** `tests/drain.rs:1938`
asserts a global `workspace create` tally of exactly 1;
`tests/drain.rs:948-954` is a byte-exact whole-transcript `assert_eq!` on
`gh` calls. Both break the moment a second reviewer launches. Three fake
shims (`tests/drain.rs:1205-1335`, `:1348-1549`, and the inline one at
`:1084`) have **no `workspace create` branch and a terminal `else … exit 2`**
— a differently-named second reviewer hard-fails the drain inside them.
Additionally, five fixtures return a single hard-coded `workspace_id`, so
two reviewers would receive the *same* id and silently defeat reap-count
assertions rather than failing loudly.

`tests/drain.rs` is therefore a second contention point alongside
`src/review.rs`, and Module G cannot be its own lane.

### Provisional modules and bundles

| Module | Location | Confidence |
|---|---|---|
| A — grammar and naming | `src/review.rs:11-31, 408-429` | HIGH |
| B — brief template and builder | `src/review.rs:336-368, 431-461` | HIGH |
| C — launch mechanics | `src/review.rs:463-575` | HIGH |
| D — **non-blocking prompt seam** | `src/lane.rs:340-456, 643-660` | **MEDIUM** |
| E — engine bookkeeping and gating | `src/main.rs:1625-1760` | HIGH |
| F — lane-state quorum seam | `src/lane.rs:84-129` | HIGH (recommend unused) |
| G — test harness | `tests/drain.rs` (2517 lines, 30 tests) | HIGH |
| H — contracts and docs | ADR 0005, `abacus-execute`, `docs/lifecycle.md`, `AGENTS.md` | HIGH |

Provisional bundles: **1** = A+B (same `src/review.rs` region; must land
after all four prerequisites). **2** = C+D (carries the D2 amendment and the
harness generalization). **3** = E (one contiguous region; splitting
guarantees conflicts). **4** = H (no code overlap, parallelizable, but a
real deliverable — `SKILL.md:46` and `:110-112` are what prevent S6).

Provisional sequencing: 4 can start immediately; 1 after the prerequisites
close; 2 third; 3 last.

### Discovery captured to jot

1. `rereview_heading` and its three `REREVIEW_HEADING_*` constants
   (`src/review.rs:14-16, 412-415`) are **dead code** — the cycle-2+ heading
   path was removed after PR 31 cycle 1 refuted it.
2. The `reap_reviewers_with_verdicts` missing-status-guard asymmetry that
   makes S1 destructive rather than merely wasteful.

Both await operator-invoked `/jot-review`.

---

## FIELD EVIDENCE — market-brief-package, 2026-08-27

Source: the resident market-brief-package session, answering directly about
what it ran today. This is the highest-authority input in the record: it is
observation of a working two-reviewer flow, not design reasoning. Where it
generalises from few data points it says so, and those caveats are preserved.

### It settles the engine question

`abacus run` once at session start (dispatched one bead), `abacus drain`
once (opened three spurious lanes and died on `agent_name_taken` — the
`ab-645` defect). **Since then 100% hand-orchestrated. The engine has not
read a PR all session.**

Consequence for **S1**: moot in market-brief-package, because nothing parses
those PRs. But the session's own words: *"this was luck, not design — I
picked `## Simplicity review` because it is a different role, before I knew
`heading_cycle` prefix-matches and ignores trailing text… your hazard is
real and worth writing into the contract explicitly, because
`## Adversarial review — cycle 3 — tech debt` is exactly the heading someone
would naturally choose."* S1 becomes a **hard naming rule in the contract**
rather than an engine fix.

### It corrects RESEARCH on parallelism

RESEARCH concluded the launch path serialises. True of the *engine*, but the
manual flow already achieves genuine concurrency, and the difference is one
flag:

```
herdr agent prompt <name> "<brief>" --wait --until working
```

`--until working` returns as soon as the prompt is **engaged**, not when the
turn finishes. Practice: create both worktrees, start both agents, prompt
both with `--until working`, then background two separate
`herdr agent wait … --until idle --until done --until blocked`. **Two
reviewers cost roughly the wall-clock of one.**

This also shrinks the eventual engine fix: `prompt_agent` (`src/lane.rs:389`)
blocks because it waits on default settle states, not because the recovery
machine is unsplittable. Out of scope here; recorded so the future epic
starts from it.

Gotcha, measured: `agent_pane_busy` ("target pane is not an available
shell") for **10-20 seconds** after workspace creation, longer under load.
Sleep 12-14s. An earlier 2-3s figure was wrong. And `--wait --until working`
is what proves the brief actually landed — without it you get the paste
race. *Trust `agent_status` and the tracker, never a pane read.*

### The load-bearing finding: a simplicity reviewer is not a reworded correctness reviewer

Verbatim: *"If you give it a severity floor it finds nothing, because 'this
is more complex than the problem needs' can never clear an executed-failure
bar. My correctness briefs had been actively suppressing exactly those
findings."*

**This inverts an assumption carried through FRAMING and RESEARCH** — that
the tech-debt reviewer inherits the shared evidence bar. It must not. The
two briefs are deliberately opposite:

| | Correctness | Simplicity |
|---|---|---|
| Output | Blockers | **Proposals, never blockers** |
| Severity floor | Yes | **None** |
| Executed failure | Required for a blocker | **Not required; speculation explicitly welcome** |
| Threat model | Every finding | n/a |
| Verdict | REFUTED / NOT REFUTED | **No verdict line** |
| Gates merge | Yes | No |

Proven simplicity-proposal shape — each proposal states **(a)** what is
removed, **(b)** which guarantee survives and *how it checked*, **(c)** rough
cost; and the report **ends with what it considered and REJECTED, and why**.
The session rates the rejected-list as valuable as the proposals: *"twice it
declined a tempting simplification because the guarantee mattered more…
that is the signal that the role is calibrated rather than cutting to look
productive."*

### Headings and adjudication, as actually used

- correctness → `## Adversarial review — cycle N` (em dash, integer, nothing after)
- simplicity → `## Simplicity review` (**no cycle number, different first token**)
- adjudication → `## Adjudication — cycle N`

**OQ-3 is answered by practice.** Simplicity gets no verdict and no cycle
number. It is adjudicated *inside* the correctness adjudication comment as a
labelled paragraph — typically "The parallel simplicity review is
adjudicated separately and does not gate this merge; its proposals are filed
as `<bead ids>`." Rationale: *"correctness governs mergeability, so the
grammar stays attached to the thing that gates. If you make simplicity emit
an adjudicatable verdict you have given it a veto you probably do not want."*

### OQ-5 is answered emphatically, by a case that could not be clearer

On a PR that was **itself a reduction**, the simplicity reviewer found where
the reduction had gone **too far** — a consolidated test whose fake accepted
two contracts and no longer pinned the exact call. **A restoration, reported
as a finding.** A reviewer optimising line count could not produce that.

Derived rule for reduction PRs: ask two questions — *did it overshoot*, and
*what adjacent bloat remains*.

Results in 4 runs (all from one day — discount accordingly): zero noise; two
structural insights correctness never surfaced (per-project `Cell` fan-out
replaced by sufficient statistics, 166 → ~18; one interception table driving
both runtime patching and its audit, deleting a bespoke AST recogniser);
plus genuinely dead constants and a dormant branch.

### On cycle-depth spirals — adding a reviewer is not the cause

*"Adding a reviewer does NOT cause the spiral; the spiral is caused by
briefs that reward finding something plus no class-level memory."* Three
controls, all exercised today:

1. Enumerate already-adjudicated finding **classes** in each brief; a new
   finding blocks only if it is a genuinely unadjudicated class, or shows an
   adjudicated one still live.
2. On the **second** instance of a class, refuse another point patch — the
   guard moves to the narrowest choke point covering the class.
3. On the **third**, stop and put the design question to the operator
   instead of opening another cycle. Hit twice today; stopped both times;
   both became operator decisions and both were right.

Also measured: reviewer full-suite reruns produced **zero** findings over
five cycles at 3-5 minutes each. Cut to focused suite plus import provenance
plus the reviewer's own probes: **wall-clock down ~40%, findings up.**

### Model selection — mechanism confirmed, untested

All reviewers today: `--kind codex`, `gpt-5.6-sol` at high reasoning (the
account default in `~/.codex/config.toml`). Never varied, so **no data on
the split question.** But the mechanism is confirmed to exist:

```
herdr agent start <name> --kind codex --pane <id> -- --model <m> -c model_reasoning_effort=<e>
```

herdr forwards trailing args after `--`; codex takes `-m/--model` and `-c`.
**Untested.** This is the answer to the seam question the epic expanded to
chase — and it needs no engine change at all when the orchestrator launches.

### Six hazards the contract must carry

**A. The OpenAI cyber-filter trap — six occurrences in one day.** Attack-verb
framing wedges a codex reviewer *mid-run with no verdict posted*. Trigger
words: *defeat, attack, bypass, circumvent, exploit, forge, hunt,
pathological*. **Not limited to security reviews** — a pure render-budget
robustness brief tripped it. Remedy ladder, in the order that actually
worked: state checks as correctness **invariants** ("for every composition
the finalized response satisfies these four properties") and use
"exercise"; if the subject is itself auth or credentials, go further and
describe checks **mechanically with no domain vocabulary at all** ("a
handler decodes a body without an `errors=` argument; confirm malformed
bytes yield 4xx not 500"). That last framing got an auth review through
after two failures *and produced the sharpest finding of the three*.
**Re-prompting the same wedged pane does not reliably recover — close the
workspace, fresh pane.**

**B. Goal language is load-bearing.** "Attempt to refute" rewards finding
something. Current wording: *"Render an honest verdict… Verdict REFUTED only
if you find at least one genuinely serious defect. A clean NOT REFUTED after
a real sweep is a successful review, not a failed one; never escalate a
minor issue to justify the effort."* Same rigour, and filter-safer.
Supersedes `ab-5lw`'s narrower clause — see the overlap note below.

**C. Author gates are not reviewer gates.** Trimming reviewer gates is
correct; copying that trim into an *author* brief cost a red CI on an
already-reviewed PR. The two rules must be visibly separate in the contract.

**D. Reviewers file beads** under the blocking-defect carve-out. Fold them
into the rework bead so there is one contract, unless the finding genuinely
outlives the PR.

**E. Verify the blocker yourself before accepting.** Reviewers have been
structurally right and detail-wrong — twice today a mechanism was correct
but a cited line or provenance claim needed correcting in the adjudication.

**F. File accepted concerns as beads in the same breath**, or they become
the next cycle's blockers.

### Overlap with `ab-xuz` must be resolved, not duplicated

`ab-xuz` (nine amendments to the canonical reviewer contract) and this
evidence come from the same operator's field practice weeks apart. Items B,
the class-memory controls, the focused-gate scoping, the security framing,
and guard relocation all appear in both. **DECOMPOSITION must not author two
competing contracts.** Recommended: `ab-xuz` remains the single owner of the
*correctness* brief; this epic owns the *simplicity* brief, the orchestrator
launch procedure, and the adjudication convention that binds them.

*Status: RESEARCH delivered and corrected by field evidence. FRAMING needs a
third rewrite — the epic is now an orchestrator-contract change, not engine
work. Awaiting operator decision on scope and tier.*

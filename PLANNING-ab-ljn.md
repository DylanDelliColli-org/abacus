# PLANNING — ab-ljn: the two-reviewer orchestrator contract

**Tier: FULL.** Restarted at FRAMING 2026-08-27 under a third scope, by
operator direction.

## Scope history — read this before anything else

This record has been reframed twice. Both earlier framings are wrong and
survive only in git history.

1. **Narrow** (`c75729e`) — "add one advisory tech-debt reviewer" as engine
   work.
2. **Stack** (`8722a7f`) — expanded to an N-reviewer engine seam with
   per-reviewer kind and model, after discovering the engine hardcodes
   `--kind codex` with no model selection.
3. **This one** — **not engine work at all.** Operator ruling 2026-08-27:
   *"abacus isn't at v1 yet and is really only being used in spirit… any
   changes we make at this point should really just be to the instructions
   an orchestrator receives from a `/abacus-execute` command. Assume that
   for now, we have an orchestrator manually launching workers and
   reviewers, rather than having the engine do it."*

The reframe was then **confirmed by field evidence**, not just accepted:
market-brief-package ran `abacus run` once and `abacus drain` once (which
died on the `ab-645` defect) and has been 100% hand-orchestrated since. The
engine has not read a PR there all session. The two-reviewer flow this epic
was going to design **already exists and works**, hand-run.

That inverts the epic's job. It is no longer *design a reviewer*. It is
**write down a working practice as a contract, before the knowledge decays
with the session that holds it.**

---

## Binding operator decisions

1. **FULL tier**, restarted from FRAMING.
2. **The simplicity reviewer is ADVISORY.** No commit status, no verdict
   line, no cycle number. Confirmed by practice — see decision 7.
3. **Pre-existing architectural findings go to `jot` only**, entering the
   pool solely through operator-invoked `/jot-review`. Corroborated by
   `AGENTS.md:112-115`, which already requires read-only review dispatches
   to state they are not bead-tracked work: *"a reviewer that follows the
   prime directive without this line leaves tracker and remote exhaust."*
4. **Both reviewers run every cycle, in parallel.** Now known achievable —
   the manual flow does it today.
5. **Two separate agents, not one wider brief.** Originally decided on the
   posture conflict; field evidence made the case far stronger (decision 6).
6. **The two briefs are deliberately OPPOSITE, not variations on one
   standard.** This supersedes the earlier assumption — carried through both
   prior framings — that the simplicity reviewer would inherit the shared
   evidence bar. Field verdict: *"If you give it a severity floor it finds
   nothing, because 'this is more complex than the problem needs' can never
   clear an executed-failure bar. My correctness briefs had been actively
   suppressing exactly those findings."*
7. **`ab-xuz` owns the correctness brief; this epic owns the simplicity
   brief, the orchestrator launch procedure, and the adjudication convention
   that binds them.** Prevents two competing contracts.
8. **No engine changes.** `src/` is out of scope. Engine defects found along
   the way are recorded, not fixed here.

---

## FRAMING

### The one-line frame

Turn a working, hand-run two-reviewer flow into a written contract in
`.claude/skills/abacus-execute/SKILL.md` plus a simplicity brief template,
so any orchestrator in any repo can run it without rediscovering it.

### User stories

- **OC-1** — An orchestrator can launch a correctness reviewer and a
  simplicity reviewer for one PR cycle and have them run **genuinely
  concurrently**, following a written procedure with no rediscovery.
- **OC-2** — The procedure carries its measured operational gotchas, so an
  orchestrator does not relearn them by failing: the `agent_pane_busy`
  window, the `--until working` engagement proof, and the rule that
  `agent_status` and the tracker are trusted over any pane read.
- **OC-3** — The simplicity reviewer's brief is written as its own contract
  with **no severity floor and no executed-failure requirement**, and states
  that speculation is welcome. A reviewer following it reports what
  correctness structurally cannot.
- **OC-4** — Every simplicity proposal states what is removed, which
  guarantee survives and *how it checked*, and rough cost. Every report ends
  with what it **considered and rejected, and why**.
- **OC-5** — On a PR that is itself a reduction, the reviewer is explicitly
  asked whether the reduction **overshot**, and what adjacent bloat remains.
  Restoration is a valid finding.
- **OC-6** — Reviewer headings cannot collide. No reviewer other than
  correctness may post a heading beginning `## Adversarial review — cycle `.
  Stated as a hard rule with its consequence, not as a naming preference.
- **OC-7** — The adjudication convention is written: one adjudication
  comment per correctness cycle in the existing byte-exact grammar, with the
  simplicity review adjudicated inside it as a labelled paragraph. Never its
  own `## Adjudication` heading, never a cycle number.
- **OC-8** — The contract carries the six field hazards (A–F below), each
  with the remedy that actually worked, so they are not rediscovered at
  cost.
- **OC-9** — An orchestrator can vary reviewer model and reasoning effort
  when it chooses to, via a documented mechanism, without any engine change.

### The PR evaluator stack

Ironed out at FRAMING by operator direction. This is the set of agents that
evaluate a PR, what each one answers, and which gate.

**Shipping in this epic — two evaluators.** This is what
market-brief-package runs today, so it is contracted from observation
rather than designed.

| Evaluator | The one question it answers | Heading | Gates? | Brief owner |
|---|---|---|---|---|
| **correctness** | Does this work, and can I make it fail? | `## Adversarial review — cycle N` | **Yes** — owns `adversarial-review` | `ab-xuz` |
| **simplicity** | Is this the right shape for what the bead asked? | `## Simplicity review` | No — advisory | **this epic** |

`simplicity` carries all three halves of the operator's original ask —
minimality, architectural strategy, and data-structure choice — per OQ-6:
they do not conflict, and the wrong data structure is usually *why* the code
is non-minimal.

**A third evaluator is under active consideration — see OQ-12.** Operator
2026-08-27: an evaluator dedicated to keeping tests focused, rated high
priority, because **test bloat is an observed problem in the autonomous
execution layer.**

**Two further shape questions are open — see OQ-10 and OQ-11.** Under scope
3 all three are cheap: adding or splitting an evaluator is another brief and
another `herdr agent start`, not engineering.

### The admission test, and why it is not "what subject does it own"

Simplicity had to be a separate agent because of **structural suppression**,
not topic coverage. The field report is explicit that the correctness brief
was *actively suppressing* those findings: "this is more complex than the
problem needs" can never clear an executed-failure bar, so the finding was
**unsayable** under that contract rather than merely crowded out.

That gives the admission criterion its sharp form: **what can a candidate
evaluator find that every existing evaluator structurally cannot?**

It also identifies the split axis as **epistemic mode, not subject matter.**
The four rows distinguishing correctness from simplicity — posture, evidence
bar, output, authority — are not four independent choices. They are one cut
appearing four times. That is why two is a resting point rather than an
arbitrary number, and why adding evaluators *by topic* within a mode needs
evidence of dilution rather than an appeal to coverage.

Applied to the obvious candidates, most fail: types and invariants are
reachable by simplicity; error handling and security are reachable by
correctness (which found a real PKCE flaw). **Performance is the one that
could not be dismissed** — a regression often cannot produce an executed
failure, and it is not a shape problem either.

**Admission rule for any future evaluator.** Recorded so the stack does not
sprawl by accretion:

1. **Only answerable against a concrete diff.** If a plan or a scope can
   answer it, it is not a PR evaluator.
2. **Not already covered** by CI, clippy, or an evaluator already in the
   stack.
3. **Unconditional** — runs every cycle on every PR. An evaluator needing a
   launch predicate ("only if types changed") is out of scope: the predicate
   is control flow the contract cannot express or verify.
4. **Read-only, one comment.** It reports; it never edits. This excludes
   `code-simplifier` outright — it applies changes autonomously, which would
   put a mutating agent in a reviewer workspace on the main checkout.

**Named candidates, deferred**, so a later session does not re-derive them:
`type-design-analyzer` (strongest — PR-scoped by design, rates
encapsulation and invariant expression, usefulness, and enforcement;
overlaps `simplicity`'s data-structure half, which is OQ-10),
`silent-failure-hunter` (good subject, hostile contract: *"call out every
instance… no matter how minor"* is the inverse of a severity floor),
`pr-test-analyzer` (overlaps both `correctness` and plan-time test
coverage), `comment-analyzer` (too narrow to earn a context per cycle).
`code-reviewer` is not a candidate — `correctness` supersedes it.

*Separate concern, noted once so it is not confused with this stack: the
plan-time agents (`sherlock`, `gaudi`, `columbo`, `victor`) and the
post-deploy validator (`rudy`) are not PR evaluators and are out of scope
here. The only reason they matter is that `sherlock` already covers design
rot and redundancy at plan time, which is an argument for keeping
`simplicity`'s brief pointed at the actual diff rather than at the
codebase at large.*

### Non-goals

1. **No engine changes.** Decision 8. Not the launch path, not the parser,
   not the reapers, not model selection in `src/`.
2. **Not the correctness brief.** `ab-xuz` owns it. This epic references it
   and must not contradict it.
3. **No commit status for simplicity**, and no adjudicatable verdict — that
   would hand it a veto.
4. **No beads for pre-existing debt.** Decision 3.
5. **Not a linter.** Mechanically checkable rules belong in CI.
6. **No cross-repo rollout as a separate act** — the contract lives in the
   skill, so it travels wherever the skill does.
7. **The correctness model split is not decided here.** No data exists; all
   reviewers to date ran one model. `OC-9` makes trying it cheap.

### Epic success metric

**A fresh orchestrator, in a repo it has not seen, can run one full
two-reviewer cycle from the contract alone** — launch both, collect both,
adjudicate both — without asking a question the contract should have
answered and without hitting a hazard the contract knows about.

This is the Fresh Agent Test applied to a procedure. It is the right measure
because the epic's entire purpose is transferring knowledge out of one
session before it decays.

### Narrowest valuable wedge

The simplicity brief template, the launch procedure, and the adjudication
convention — written into `.claude/skills/abacus-execute/SKILL.md` and a
brief template beside it. Nothing else.

### Prerequisites

**Changed by decision 7 and decision 8.** Under the previous framing this
epic blocked on four beads. Two were engine-side and no longer apply.

- **`ab-xuz`** — *nine amendments to the canonical adversarial-review
  contract.* **Still blocks.** It owns the correctness brief; the
  adjudication convention here binds the two reviewers together and must not
  contradict it.
- **`ab-cye`** — *verdict heading must be the first body line.* **No longer
  blocks.** Engine-side parsing; this epic touches no parser. `OC-6` handles
  the collision at the contract level instead.
- **`ab-645`** — *`sanitize_agent_name` truncation collides.* **No longer
  blocks.** Engine-side naming; the orchestrator names its own agents.
- **`ab-5lw`** — *verdict-neutrality clause.* **No longer blocks, and
  should be reconsidered.** It is correctness-brief content, so decision 7
  puts it in `ab-xuz`'s territory — and field hazard B supersedes its clause
  with better wording. **Recommendation: fold `ab-5lw` into `ab-xuz` and
  close it.** Flagged for the gate; not acted on.

---

## Open questions

- **OQ-3 — advisory verdict grammar.** RESOLVED by practice: no verdict, no
  cycle number, adjudicated inside the correctness comment.
- **OQ-5 — is TD-5 a known anti-pattern?** RESOLVED. Line count is dead as a
  target; `OC-5` carries the intent. See the restoration case below.
- **OQ-6 — one simplicity agent or several?** RESOLVED: one. Field practice
  runs one and it works; RESEARCH adds the principled distinction. OQ-4
  split correctness from simplicity because those mandates **conflict** — a
  correctness finding demands more code, minimality demands less, and one
  context must arbitrate silently. Minimality, architectural strategy, and
  data-structure choice **do not conflict**; they are three probes of one
  question ("is this the right shape for what the bead asked?"), and a
  finding in one routinely *is* a finding in another — the wrong data
  structure is usually *why* the code is non-minimal. Splitting would
  produce three reports restating one finding from three angles, which is
  worse for the operator. Caveat to carry: if `type-design-analyzer` ever
  joins the stack it overlaps OC-3's data-structure half; decide then
  whether OC-3 narrows, rather than shipping both without noticing.
- **OQ-7 — epic success metric.** SUPERSEDED by the Fresh-Orchestrator
  metric above.
- **OQ-8 — should `ab-5lw` be folded into `ab-xuz` and closed?** Open;
  planner recommends yes.
- **OQ-10 — does `simplicity` keep the data-structure/type mandate, or does
  it split into a second advisory evaluator?** Open. OQ-6 resolved that the
  three halves do not conflict, which argues for one. Against:
  `type-design-analyzer` is a PR-scoped agent with a four-axis rating shape
  that is more specific than anything `simplicity`'s brief will say about
  types, and field evidence for `simplicity` is 4 runs from one day — thin
  enough that the bundle is unproven rather than proven. Planner
  recommendation: **keep one**, ship it, and split only on observation.
- **OQ-11 — does `correctness` split into two targeted evaluators?** Open,
  and reopened by scope 3. Under the retired engine framing this needed a
  model/kind seam and was deferred as a measured experiment. Under
  orchestrator control it needs neither — it is two briefs and two launches.
  The operator's original framing was "two faster, less expensive models
  with more targeted instructions." Still no measurement exists, but the
  cost of trying is now a brief rather than an epic.
- **OQ-12 — a third evaluator for test focus?** Raised by the operator
  2026-08-27 and rated high priority: **test bloat is an observed problem in
  the autonomous execution layer.** Under discussion with a Codex peer;
  operator rules at the decision point.

  Planner position, which **reverses an earlier dismissal.** My first
  instinct was that simplicity covers it, since tests are code and
  "non-minimal" applies. That is wrong for four reasons:

  1. **The proposed OC-5 metric is blind to it.** Counting *concepts* was
     chosen because it is not gameable by density — but forty near-identical
     parameterised test cases are **one concept and forty tests.** The
     metric cannot see the bloat it would need to report.
  2. **Tests have a different unit of analysis.** Source asks "fewer
     concepts." Tests ask "does each case pin a *distinct* behaviour, and is
     the assertion load-bearing?" A suite can be conceptually lean and still
     bloated.
  3. **The mandate is two-sided, unlike simplicity's.** A test evaluator
     must cut redundancy *and* protect against coverage loss — the three
     named classes being cases folded until distinct behaviour is no longer
     proved, tests deleted while their production behaviour is live, and
     assertions thinned inside surviving test names. Simplicity only ever
     cuts.
  4. **There is a standard to review against, unused at review time.**
     `FULL_SUITE_WALL_CLOCK_BUDGET_SECONDS = 30`, and `columbo --cost-audit`
     ranks tests by duration against unique coverage — but only at plan
     time. Nothing checks it against an actual diff.

  On the structural test, a test evaluator therefore **passes**, and the
  earlier dismissal was wrong.

  **Codex peer discussion, 2026-08-27 (`w84:p3`). Joint recommendation:
  three evaluators now.** Points of agreement and the one correction:

  - **The admission test needed widening, and Codex's version is better.**
    Structural suppression is *sufficient* for admission but too strict as
    the *only* route: "could another agent theoretically report this" is not
    "will its contract reliably direct enough attention there." Proposed
    second route: **demonstrated attention dilution** — repeated misses,
    recurring operator pain, or a focused trial producing material findings
    the broad evaluator routinely overlooks. Empirical, not asserted, so
    dilution does not become permission to split by subject area.
  - **Argument 3 above is wrong and is withdrawn.** Simplicity is *already*
    two-sided: every proposal must preserve a checked guarantee, the
    rejected list records tempting cuts that would not, and the restoration
    case proves it can recommend adding something back. Cut-and-protect
    therefore does **not** distinguish the roles. Arguments 1, 2, and 4
    stand and are what carry the case.
  - **Sharper framing than the planner's:** test review operates over a
    **behaviour-to-evidence relationship**, not over code shape. It asks
    whether each case proves a distinct behaviour, whether each assertion is
    load-bearing, whether a fake still pins the exact contract, and whether
    production behaviour lost its proof through deletion, folding, or
    assertion thinning.
  - **Scope note with a real cost:** the test evaluator must examine tests
    **even when no test file changed**, because a production change can make
    existing evidence incomplete or meaningless. It reads the production
    diff *plus* the implicated existing suite — more context than the other
    two carry.
  - **Simplicity narrows:** its test responsibility drops to incidental
    observation rather than test-suite design as part of its required sweep.
    Resolves the overlap cleanly.
  - **Redundancy and coverage loss are reported separately** by the test
    evaluator — they are opposite findings and folding them invites the
    adjudicator to trade one against the other silently.

  **The ceiling, and what sets it.**

  - **Exactly one gating evaluator.** Two independent gates would duplicate
    severity decisions and increase cycle depth even when launches are
    parallel. A second gate needs a genuinely separate *authority boundary*,
    not merely specialised expertise — and none has been shown.
  - **Advisory capacity has a softer ceiling but is not free.** Parallel
    launch removes wall-clock cost, not token cost, comment-reading cost,
    adjudication cost, or pressure to manufacture findings. **Three total is
    the resting point; four is an experimental ceiling requiring measured
    justification.** Six would be "a taxonomy, not a review design" — the
    operator would receive overlapping comments about one diff.
  - **Performance is the only credible fourth mode**, and both parties agree
    **not now**: without evidence of recurring performance escapes it would
    spend most cycles proving irrelevance. Trial it as advisory with
    explicit baselines and budgets if such regressions become observed.
  - No other unconditional evaluator passes either route today. Types, error
    handling, security, API compatibility, requirements fidelity,
    observability, and documentation all remain reachable through
    correctness or simplicity absent field evidence of systematic misses.
- **OQ-9 — does the contract belong only in the skill, or also in an ADR?**
  Open. The skill travels and is where an orchestrator reads; an ADR is
  binding and survives skill rewrites. Deferred to RECORD, which is
  artifact-conditional by design.

---

## FIELD EVIDENCE — market-brief-package, 2026-08-27

The highest-authority input in this record: observation of a working
two-reviewer flow, not design reasoning. Caveats preserved where the source
gave them.

### Launch procedure, as actually run

```
herdr worktree create --cwd <repo> --branch review/prNN-cK \
      --base origin/lane/<bead> --label review-prNN-cK   → .result.root_pane.pane_id
sleep 12-14                                              ← see gotcha
herdr agent start codex-prNN-rK --kind codex --pane <pane_id>
herdr agent prompt codex-prNN-rK "<brief>" --wait --until working
# then, backgrounded, one per reviewer:
herdr agent wait codex-prNN-rK --until idle --until done --until blocked
```

**Concurrency comes from `--until working`**, which returns as soon as the
prompt is *engaged* rather than when the turn finishes. Create both
worktrees, start both agents, prompt both, then background two separate
waits. **Two reviewers cost roughly the wall-clock of one.**

**Gotcha, measured:** `agent_pane_busy` ("target pane is not an available
shell") for **10-20 seconds** after workspace creation, longer under load.
Sleep 12-14s; an earlier 2-3s figure was wrong. `--wait --until working` is
also what proves the brief landed — without it you get the paste race.
**Trust `agent_status` and the tracker, never a pane read.**

### The two briefs are opposite

| | Correctness | Simplicity |
|---|---|---|
| Output | Blockers | **Proposals, never blockers** |
| Severity floor | Yes | **None** |
| Executed failure | Required for a blocker | **Not required; speculation welcome** |
| Threat model | Every finding | n/a |
| Verdict | REFUTED / NOT REFUTED | **No verdict line** |
| Gates merge | Yes | No |

Simplicity proposal shape: **(a)** what is removed, **(b)** which guarantee
survives and *how it checked*, **(c)** rough cost. The report **ends with
what it considered and REJECTED, and why** — rated as valuable as the
proposals, because *"twice it declined a tempting simplification because the
guarantee mattered more… that is the signal that the role is calibrated
rather than cutting to look productive."*

### Headings and adjudication, as used

- correctness → `## Adversarial review — cycle N` (em dash, integer, nothing after)
- simplicity → `## Simplicity review` (no cycle number, different first token)
- adjudication → `## Adjudication — cycle N`

Simplicity is adjudicated inside the correctness adjudication comment as a
labelled paragraph: *"The parallel simplicity review is adjudicated
separately and does not gate this merge; its proposals are filed as
`<bead ids>`."* Rationale: *"correctness governs mergeability, so the
grammar stays attached to the thing that gates. If you make simplicity emit
an adjudicatable verdict you have given it a veto you probably do not
want."*

The heading collision was avoided **by luck, not design** — the source
picked `## Simplicity review` before knowing `heading_cycle` prefix-matches
and ignores trailing text, and asks that the hazard be written into the
contract explicitly, *"because `## Adversarial review — cycle 3 — tech debt`
is exactly the heading someone would naturally choose."* Hence `OC-6`.

### Results — 4 runs, one day, discount accordingly

Zero noise. Two structural insights correctness never surfaced: per-project
`Cell` fan-out replaced by sufficient statistics (166 → ~18), and one
interception table driving both runtime patching and its audit, deleting a
bespoke AST recogniser. Plus dead constants and a dormant branch.

**The restoration case, which settles OQ-5.** On a PR that was *itself a
reduction*, the reviewer found where the reduction had gone **too far** — a
consolidated test whose fake accepted two contracts and no longer pinned the
exact call. A restoration, reported as a finding. **A reviewer optimising
line count cannot produce that.**

Cost: 10-20 min per reviewer; two in parallel ≈ one; roughly double tokens.

### Cycle-depth spirals — the reviewer is not the cause

*"Adding a reviewer does NOT cause the spiral; the spiral is caused by
briefs that reward finding something plus no class-level memory."* Three
controls, all exercised:

1. Enumerate already-adjudicated finding **classes** in each brief; a new
   finding blocks only if genuinely unadjudicated, or shows an adjudicated
   class still live.
2. On the **second** instance of a class, refuse another point patch — the
   guard moves to the narrowest choke point covering the class.
3. On the **third**, stop and put the design question to the operator.
   Hit twice; stopped both times; both were right.

Measured: reviewer full-suite reruns produced **zero** findings over five
cycles at 3-5 min each. Cut to focused suite + import provenance + the
reviewer's own probes: **wall-clock down ~40%, findings up.**

### Model selection — mechanism confirmed, untested

All reviewers to date: `--kind codex`, `gpt-5.6-sol` at high reasoning.
Never varied, so **no data on the split question.** Mechanism:

```
herdr agent start <name> --kind codex --pane <id> -- --model <m> -c model_reasoning_effort=<e>
```

herdr forwards trailing args after `--`; codex takes `-m/--model` and `-c`.
Needs no engine change under manual orchestration. This is `OC-9`.

**CONFIRMED 2026-08-27 by RESEARCH**, upgrading this from "mechanism exists,
untested". `strings` on the herdr binary surfaces its own embedded operator
guidance, verbatim: *"Pass native agent arguments only after `--`"*, with
the worked example `herdr agent start reviewer --kind codex --pane <id> --
<agent-args...>` — literally our case. Corroborated by the binary's error
strings `invalid_agent_argument` and *"agent arguments cannot be encoded
safely for the target shell"*, which exist only if herdr shell-encodes
AGENT_ARG onto the launched command line. Codex side: `codex --help:75-76`
documents `-m, --model <MODEL>`.

Two constraints for the contract: **prefer `-m` over `-c`** (`-c` parses its
value as TOML with a raw-string fallback — a needless parsing surface for a
model name); and **model strings must survive herdr's shell encoding** —
alphanumerics and dashes are safe, anything exotic returns
`invalid_agent_argument`.

Residual uncertainty, narrow: nobody has observed a codex process actually
receiving the flag. That is a confirmatory smoke test at implementation, not
a planning risk. Per-reviewer **kind** selection carries no risk at all —
`--kind` is a first-class herdr flag.

### Six hazards the contract must carry

**A. The OpenAI cyber-filter trap — six occurrences in one day.**
Attack-verb framing wedges a codex reviewer *mid-run, no verdict posted*.
Triggers: *defeat, attack, bypass, circumvent, exploit, forge, hunt,
pathological*. **Not limited to security reviews** — a render-budget
robustness brief tripped it. Remedy ladder, in the order that worked: state
checks as correctness **invariants** and use "exercise"; if the subject is
auth or credentials, describe checks **mechanically with no domain
vocabulary at all** (*"a handler decodes a body without an `errors=`
argument; confirm malformed bytes yield 4xx not 500"*). That framing got an
auth review through after two failures *and produced the sharpest finding of
the three*. **Re-prompting a wedged pane does not reliably recover — close
the workspace, fresh pane.**

**B. Goal language is load-bearing.** *"Render an honest verdict… Verdict
REFUTED only if you find at least one genuinely serious defect. A clean NOT
REFUTED after a real sweep is a successful review, not a failed one; never
escalate a minor issue to justify the effort."* Same rigour, filter-safer.
Correctness territory — belongs to `ab-xuz`; supersedes `ab-5lw`.

**C. Author gates are not reviewer gates.** Trimming reviewer gates is
correct; copying that trim into an *author* brief cost a red CI on an
already-reviewed PR. Keep the two visibly separate.

**D. Reviewers file beads** under the blocking-defect carve-out. Fold into
the rework bead unless the finding outlives the PR.

**E. Verify the blocker yourself before accepting.** Reviewers have been
structurally right and detail-wrong — twice a mechanism was correct but a
cited line or provenance claim needed correcting in the adjudication.

**F. File accepted concerns as beads in the same breath**, or they become
the next cycle's blockers.

---

## Contract content RESEARCH contributed

Findings that survive the reframe and belong in the simplicity brief. From
the scope-2 RESEARCH passes, which remain valid where they concern brief
content rather than engine code.

### OC-5, proposed wording for ratification

Refines the planner's "unnecessary code, not line count" with a countable
unit. **Count concepts, not lines** — a concept being a name a reader must
hold: a type, a trait, an abstraction layer, a configuration knob, a
control-flow branch, a file. Unlike line count it is not gameable by
density: you cannot make a concept disappear by writing a nested ternary.

> **OC-5** — The reviewer targets unnecessary code and unnecessary
> **concepts** — names a reader must hold: types, abstraction layers,
> configuration knobs, control-flow branches, files. Line count is not the
> objective and is never cited as a reason. A change that adds lines while
> removing a concept is a valid simplification, as is one that adds lines to
> eliminate a nested ternary or a dense one-liner. **If a proposed
> simplification would require changing an existing test, it is a behaviour
> change, not a simplification** — report it as such or drop it. A proposal
> that increases both lines and concepts must state explicitly why; an
> unjustified one is a defect in the review.

Supporting evidence that the original wording was an anti-pattern:
Anthropic's own `code-simplifier.md:38` classifies *"Prioritize 'fewer
lines' over readability (e.g., nested ternaries, dense one-liners)"* as an
**over-simplification failure** — precisely what a reviewer incentivised to
cut produces. A numeric target is one the reviewer can always hit by
proposing density.

### Finding volume: threshold and ordering, never a cap

Resolves the collision between the external sets' top-3 cap and `ab-xuz`
amendment 1's exhaustive sweep. **The resolution is principled, not
convenient: exhaustive sweep is a consequence of GATING, not of
thoroughness.** Each unenumerated *correctness* finding costs a full extra
REFUTED cycle — relaunch, rework dispatch, re-review, re-adjudication,
10-18 minutes — *because the findings gate*. An advisory reviewer's
unreported finding costs nothing but the finding. **The mandate does not
transfer, and the brief must say so in those terms**, or a future session
will "fix" the inconsistency and reintroduce it.

But do not adopt a numeric cap either: **a fixed cap has the identical
defect to a line-count target — it is a quota the reviewer will fill.** Use
instead:

- **A significance threshold** — report nothing below it. Mechanism borrowed
  from `code-reviewer.md:41` ("only report issues with confidence ≥ 80").
  *Copy the mechanism, not the numbers* — that file's bands and gate are
  internally inconsistent as shipped.
- **An explicit "reporting nothing is a valid and expected outcome"
  clause** — the advisory analogue of the goal-language hazard (B). Without
  it, a reviewer with no quota still manufactures findings.
- **An exclusion list** — tests, generated code, vendored deps, sub-5-line
  snippets, language idioms; plus one entry no external list has and this
  contract needs: **never propose removing code the bead's own acceptance
  criteria require.**
- **Ordering, not capping** — report findings ranked by significance, most
  significant first. An operator who reads three and stops has read the
  three that mattered: the cap's benefit with none of its quota pressure.

### Membership is unconditional

`commands/review-pr.md:36-43` shows the pattern to avoid — *"If test files
changed: pr-test-analyzer"*, *"If types added/modified:
type-design-analyzer"*. A launch predicate makes the procedure conditional
and the contract unverifiable. State in the contract: **every reviewer runs
every cycle; a reviewer requiring a launch predicate is out of scope.** This
is already operator decision 4, but consistent for a reason nobody had
written down.

### The external set, finally assessed

**Reusable:** `comment-analyzer.md:79`'s explicit read-only clause as a
posture sentence (*"You analyze and provide feedback only… Your role is
advisory"*); `code-reviewer.md:41`'s single-number threshold as a mechanism;
`type-design-analyzer.md:60-87`'s fenced-template discipline and four-axis
rating shape (Encapsulation, Invariant Expression, Invariant Usefulness,
Invariant Enforcement) as a model for OC-3; `pr-test-analyzer.md:40, :72`'s
false-positive guards; the `agentic-awesome-skills` test red flag, now
folded into OC-5.

**Actively wrong for this contract:** `silent-failure-hunter.md:114`'s *"no
matter how minor"*, the direct inverse of a severity floor; **the entire set
has no evidence bar** — not one of the six requires reproducing or executing
anything before reporting, so their findings are counterfactual by
construction; `code-simplifier` edits autonomously; and several hardcode one
codebase's standards as universal (`code-simplifier.md:15-20` is
ES-modules/React; `silent-failure-hunter.md:123-128` names another repo's
helpers). **OC-3's "against the patterns already in this repository" is
stronger than anything in the external set.**

**Useful negative:** no source surveyed, internal or external, has prior art
for multi-reviewer bookkeeping or any coordination protocol between
reviewers. `commands/review-pr.md:45-55` names sequential-versus-parallel as
advisory prose with no mechanism, and its aggregate vocabulary matches none
of its own agents' vocabularies. The convention this epic writes is
unprecedented in every source checked.

### Citation defect found and corrected

**"ADR 0003 D10" does not exist.** ADR 0003 has no bold-D decision headings
at all; the no-config-file decision is in its *"Not built now"* list at
`docs/adr/0003-pr-validation-and-auto-merge.md:205-215` as *"a configuration
file (opt-in stays invocation)"*. Verified this session.

This matters more than a dangling reference: a `D10` **does** exist, at
`docs/adr/0004-foreign-repo-onboarding.md:165` ("Two-session negotiation
with seat-scoped authority"), so a future session greping for it lands on a
real but unrelated decision. `ab-init-plan-5ka`'s description propagates the
wrong label; a correcting comment was added to that bead 2026-08-27.

Related: `ab-init-plan-5ka` **already claims the per-repo config surface and
already names "agent kind?" as a candidate item.** Scope 3 avoids the
collision naturally — this epic defines no configuration file, because the
orchestrator passes flags by hand. Recorded so a future engine epic does not
walk into it.

---

## Superseded: engine RESEARCH

A full engine audit was produced at `fb2c106` and is **superseded by
decision 8** — this epic touches no engine code. It retains durable value
for a future engine epic and should not be re-derived. Highlights:

- **S1, critical and reachable.** `heading_cycle` (`src/review.rs:97-101`)
  prefix-matches and ignores trailing text, so a colliding heading registers
  a phantom cycle, which kills the live correctness reviewer via an
  unguarded `workspace close` (`src/main.rs:1678-1696`) and then suppresses
  relaunch (`:1728-1730`). An operator adjudicating that phantom cycle can
  flip `adversarial-review` to success and clear a PR to merge with **zero
  correctness review performed.** Mitigated *in this epic* by `OC-6`; the
  engine defect remains.
- **37 single-reviewer assumptions** enumerated: 17 in code, 5 in contracts,
  15 in tests.
- **The engine serialises reviewers** because `prompt_agent`
  (`src/lane.rs:389`) waits on default settle states. Field evidence shows
  the fix is likely `--until working`, not a restructure.
- Prior art: **every ADR in this repo was reviewed by two independent
  reviewers** — a bloat review and a spec validation. The bloat review's
  per-cut *"Cost of cutting" / "Revive when"* form is a native, field-proven
  shape for simplicity proposals.

### Discovery captured to jot

`rereview_heading` and its constants (`src/review.rs:14-16, 412-415`) are
dead code. And the `reap_reviewers_with_verdicts` missing-status-guard
asymmetry underlying S1. Both await `/jot-review`.

*Status: FRAMING rewritten under scope 3, awaiting operator gate. RESEARCH
to be re-run scoped to the contract once FRAMING is approved.*

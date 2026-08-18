```doc-meta
role: working
lifecycle: inflight
```

# PLANNING — ab-lifecycle-v2-go4: Lane lifecycle v2

**Tier: FULL**, operator-confirmed 2026-08-18. Rationale: the cluster
introduces new contracts (an engine lane-outcome model, a required
`adversarial-review` GitHub status check, the two-comment PR verdict
convention, a templated refutation brief); it contains a real architecture
decision (the engine is currently strictly serial and stateless — warm lanes
awaiting review while a drain continues means concurrent live lanes tracked
across invocations); and it carries two live unknowns (the `herdr agent wait`
split-pane false-fail boundary; the check-flip auth identity, where today's
default-branch flip already required the DDC-Heartwood account). Input specs:
`ab-phr`, `ab-co5`, `ab-blocked-lane-outcome-6bs` — each operator-directed,
each carrying production precedent from market-brief-package 2026-08-17/18
(PRs 23–26).

## FRAMING

Drafted by the orchestrator from the operator's recorded rulings in the three
input beads; presented live for the FRAMING gate 2026-08-18.

### User stories

- **S1 — automatic adversarial review.** After a lane settles with a PR, an
  adversarial reviewer launches automatically as a fresh codex context in its
  own dedicated herdr workspace, handed a refutation brief generated from the
  bead spec, so every lane PR receives refutation-grade review without manual
  choreography. (From `ab-phr`; pane choreography per bb-skills
  `agents/LAUNCHING.md`.)
- **S2 — durable two-comment verdicts.** The reviewer posts its full
  unadjudicated verdict as one PR comment (its only permitted write); after
  the operator rules, the orchestrating side posts the adjudication
  follow-up. Raw verdicts are durable evidence; nothing blocks on them until
  adjudicated. (From the `ab-phr` amendment; precedent backfilled on
  market-brief PRs 25/26.)
- **S3 — structural enforcement.** Lane PRs carry a required
  `adversarial-review` status check that stays pending until the adjudication
  comment flips it, so an unreviewed PR cannot merge even by habit. (From
  `ab-phr`.)
- **S4 — warm rework.** When the operator's ruling requires rework, the
  rework spec is prompted into the SAME warm worker on the same lane branch;
  the lane reaps on merge or explicit abandon, not on settle. Measured basis:
  fresh rework lanes cost 8–15 min vs ~5 min warm (one bead paid the
  orientation tax three times). (From `ab-co5`.)
- **S5 — drain resilience.** An unattended drain continues past lanes that
  settle blocked or awaiting-review instead of aborting the night, and a
  reopened-for-rework bead redispatches into its warm agent rather than
  minting a fresh lane. (From `ab-blocked-lane-outcome-6bs` + `ab-co5`;
  production basis: three same-branch rework cycles clean under single
  `abacus run` invocations, so the gap is drain-specific.)
- **S6 — legible morning report.** Every lane settle is reported by class —
  completed / blocked / awaiting-review / reopened-rework / stalled — so the
  operator reads one report instead of opening panes. (From
  `ab-blocked-lane-outcome-6bs`.)

### Non-goals

- **N1 — no CI-authoritative review.** Operator considered and rejected
  2026-08-18: CI loses the jot funnel and the operator interrogation
  surface. The optional non-blocking CI commenter mentioned in `ab-phr` is
  out of this epic entirely; any API-key-in-CI variant needs a fresh operator
  ruling.
- **N2 — no API-key reviewer auth.** Codex subscription OAuth only (standing
  ruling 2026-07-20).
- **N3 — no warm reviewers.** Reviewers are ephemeral, one fresh context per
  review cycle. The author-warm / reviewer-ephemeral asymmetry is deliberate
  design, not an optimization target.
- **N4 — no automated adjudication.** The operator rules on findings after
  verifying them against real code; machinery only records verdicts, flips
  the check after the ruling, and routes rework.
- **N5 — no BLOCKED-lane resume machinery.** A blocked lane parks with its
  durable comment; un-blocking remains manual. (Warm rework under S4 applies
  to review-driven rework, not to blocked lanes.)
- **N6 — no general parallel-dispatch scaling and no bundling.** Concurrency
  enters only as far as the cluster requires: settled lanes may await
  adjudication while the drain proceeds. Cluster/bundle selection stays with
  `ab-bundling-plan-4mu` (a separate parked planning trigger for engine-side
  lane bundling).

### Epic success metric

An unattended `abacus drain` over at least three ready beads in an onboarded
repository ends with: every settled lane classified (zero drain aborts from
blocked or awaiting-review states), every lane PR carrying a reviewer verdict
comment and a pending required check, and any rework cycle dispatched into a
warm agent — with zero manual pane interventions between first dispatch and
the morning report.

### Narrowest valuable wedge

The outcome model plus drain continuation (`ab-blocked-lane-outcome-6bs`
scope): the drain classifies blocked / awaiting-review / rework settles and
continues instead of aborting, with per-class reporting. This ships value
alone — it converts the observed drain-killing failure into a parked lane —
and every later story builds on its classification. Review-gate automation
(S1–S3) and warm-lane keepalive (S4) layer on top.

### Prerequisites

None. The two live unknowns (split-pane `agent wait` boundary; check-flip
auth identity) are RESEARCH items inside this run, not blocking beads.
`ab-24o` (sandbox worktree-index bead, filed by another session) is judged
unrelated pending RESEARCH confirmation.

---

*FRAMING approved by operator 2026-08-18.*

## RESEARCH

Produced by a sherlock-type subagent (lifecycle-research), delivered
2026-08-18; integrated verbatim by the orchestrator. Audited at abacus HEAD
`c22073e`. All source reads read-only; the single scratch herdr workspace
(w34) was created for the Unknown-A experiment and removed (verified). No
beads created or mutated; all gh probes read-only.

### 1. Engine fingerprints

**Confidence: high** (direct read of all three source files, full test suite
executed).

**Dispatch/outcome seams** — everything the outcome-model and drain work
touches:

- `BeadOutcome` (src/lib.rs:82) — three variants: `Completed`, `Incomplete`,
  `NeverEngaged`. Classified purely from bead status by
  `classify_bead_status` (src/lib.rs:88). `parse_bead_outcome`
  (src/lib.rs:126) parses `br show <id> --json` and deserializes ONLY
  `status`; comments are not fetched or modeled anywhere yet, so the Blocked
  signal ("most recent comment begins with BLOCKED") needs a widened parse,
  new fixture shapes, and the max-comment-id ordering rule from the bead
  spec.
- `probe_bead_outcome` (src/main.rs:708) — shells `br show --json` with one
  2s-delayed retry via `retry_probe_once` (src/main.rs:694).
- `dispatch_cycle` (src/main.rs:824) — the whole lane lifecycle inline:
  select → claim → `herdr worktree create` → `agent start --kind codex` →
  `dispatch_prompt` via `agent prompt --wait` (one stall retry keyed on
  `is_agent_prompt_stalled`, src/lib.rs:107) → `probe_bead_outcome` → one
  never-engaged re-prompt (`retry_never_engaged_once`, src/main.rs:676) →
  conditional reap → final outcome match (src/main.rs:946) where
  `Incomplete` and `NeverEngaged` become `Err` — exactly what aborts a
  drain.
- `cmd_run` (src/main.rs:730) / `cmd_drain` (src/main.rs:743) — drain loops
  `dispatch_cycle`, tracks only `lost_claims: BTreeSet<String>` in memory;
  ANY `Err` propagates through `?` and kills the loop (the observed
  drain-abort mechanism, matching ab-blocked-lane-outcome-6bs).
- **Reap logic** — `should_reap_lane` (src/lib.rs:100): reap iff
  `Completed`. The reap block (src/main.rs:919–944) calls `herdr worktree
  remove --workspace`, with a dirty-checkout escalation keyed on
  `is_dirty_worktree_remove_error` (src/lib.rs:115) that forces removal
  with a warning. The 6bs "reap blocked lane only if clean" item can reuse
  this error-code discrimination directly — attempt non-forced remove; on
  dirty error, leave and report (inverting the current force behavior for
  the Blocked class).
- `dispatch_prompt` (src/lib.rs:182) — already ends with the BLOCKED escape
  hatch but does NOT instruct the durable `br comments add "BLOCKED: ..."`
  step; that lives in the target repo's AGENTS.md (verified:
  market-brief-package AGENTS.md step 7). Engine-side signal and deployed
  contract agree on the leading token "BLOCKED".

**Statelessness (explicit):** the engine persists nothing between
invocations — `lost_claims` is in-memory per call; everything else is
re-derived per cycle from `br`/`gh`/`herdr` output. Implication for warm
lanes: within one drain invocation, tracking live lanes is another in-memory
set; across invocations (morning rerun, crash — the operator host is
crash-prone, so recovery is first-class), warm-lane state must be
RE-DERIVABLE, and it is: agent name is deterministic
(`sanitize_agent_name(bead_id)`, src/lib.rs:57), branch is deterministic
(`lane/{bead_id}`, src/main.rs:853), and `herdr agent list` /
`herdr workspace list` + `br show` + `gh pr view` jointly reconstruct any
lane's state with zero persistence. A stateless re-derivation design is
available and fits the engine's character; whether to take it is an
ARCHITECTURE decision, but nothing in RESEARCH forces a state file.

**land.rs reuse assessment** (PR-comment posting, status reads/writes,
PR-state watching):

- PR-comment posting: `comment_on_candidate` (src/main.rs:247) — `gh pr
  comment <branch> --body` — is exactly the adjudication-comment primitive;
  pure body-builder precedent in `admission_red_park_body` /
  `dequeue_park_body` (src/land.rs:401/414). Directly reusable.
- PR-state watching: `observe_queue_state` + `QUEUE_QUERY` GraphQL
  (src/main.rs:436–478) and `parse_queue_state` (src/land.rs:355) already
  watch merged/open states per branch; `enumerate_candidates`
  (src/land.rs:125) already intersects open `lane/*` PRs with bead state.
  Reap-on-merge detection can reuse the Merged classification nearly
  verbatim (a lighter REST `gh pr view --json state,mergedAt` also
  suffices).
- Status checks: NO existing machinery — `parse_eligibility`
  (src/land.rs:34) reads rulesets only. Commit-status POST/read is new but
  small, in the established `capture` style (src/main.rs:972).
- Reviewer-lane launch: `dispatch_resolution` (src/main.rs:310–354) is the
  closest template — the reviewer variant swaps worktree-open for a
  dedicated workspace and a brief file path.

**Test surface:** tests/ has 6 integration files. br_roundtrip.rs (1591
lines, 23 tests) is the real-temp-workspace pattern (`TempWorkspace` at
tests/br_roundtrip.rs:16, real `git init` + real `br` via `require_br!` at
line 120, real `abacus` via `CARGO_BIN_EXE_abacus`). drain.rs (140 lines, 1
test) is the OTHER pattern — fake `br`/`herdr`/`git` shell shims on a
prepended PATH (tests/drain.rs:41–97) — and is the pattern for
drain-continuation tests since no live herdr is needed. Unit tests: 34 in
lib.rs, 12 in main.rs. **Measured full-suite duration: 7.86s wall on a warm
build; br_roundtrip alone 6.7s; ~89 tests.** One flake observed: in one of
four full-suite runs br_roundtrip finished 22 passed / 1 failed; the same
suite passed 23/23 in six other runs; the failing test's name was lost
before its significance registered. At least one intermittent br_roundtrip
flake exists, identity unknown, roughly 1-in-4 at this sample —
TEST-STRATEGY should budget for identifying it before this cluster piles
~10 more integration tests onto the same file. (Captured to the jot funnel
by the orchestrator at integration.)

### 2. UNKNOWN A — herdr monitoring boundary: RESOLVED

**Confidence: high for codex-kind agents on the current herdr build;
untested for claude-kind.** Method: scratch workspace w34 (root pane w34:p1,
split pane w34:p2), one codex agent per pane, trivial prompts, timed
foreground and background probes; workspace removed after.

Observed codex status lifecycle under herdr: **idle (pre-first-turn) →
working → done (settled). A codex agent never returns to "idle" after
completing a turn.** Every result follows:

| Probe | Split pane (w34:p2) | Dedicated root pane (w34:p1) |
|---|---|---|
| `agent prompt --wait` | exit 0, 4.8s, settles "done" | exit 0, 1.8s, settles "done" |
| `agent wait --until idle` vs settled agent | exit 1, timeout (30s cap) | exit 1, timeout (45s cap) |
| `agent wait` default states vs settled agent | exit 0 in 0.002s (matched "done") | — |
| `agent wait --until done` across working→done | — | exit 0, fired at 13.7s |
| `agent wait --until idle` vs PRE-first-turn agent | — | exit 0 instantly (genuinely "idle") |

**Boundary statement:** `herdr agent wait --until idle` fails against codex
agents in BOTH topologies — the split-pane hypothesis in ab-co5 is refuted;
topology has no effect on any command tested. The production false-fails are
fully explained: `--until idle` never matches a settled codex agent, an
untimed wait runs until its target dies, and the death error is
`agent_not_running`. The 1-of-7 success: a wait starting while the agent
still reads pre-first-turn "idle" matches instantly. **Working commands:
`agent prompt --wait` (both topologies), and standalone `agent wait` with
default states or `--until done`.** The dedicated-workspace standard stands
as pane hygiene, not a monitoring requirement. Caveats: n=1 per cell, codex
kind only, current herdr build only; a claude-kind agent's settle status
name must be re-verified before reusing `--until done`.

Corollary: bb-skills/agents/LAUNCHING.md step 4 embeds the broken
`--until idle` invocation — the "proven choreography" was proven with the
operator manually reading panes. The automated version must not copy that
line. (Doc fix belongs to bb-skills; captured to the jot funnel.)

### 3. UNKNOWN B — check-flip auth: RESOLVED, with a plan-gate surprise

**Confidence: high on permissions and plan gate (probe-verified); high on
statuses-vs-checks mechanics (documented behavior corroborated by read-only
observation; the actual POST was not exercised).** DDC-Heartwood token used
via `GH_TOKEN=$(gh auth token --user DDC-Heartwood)` without switching the
shared active account.

- Repo permissions on DDC-Heartwood/market-intelligence: DylanDelliColli
  `push=true, admin=false`; DDC-Heartwood `admin=true`. Repo is **private,
  User-owned, free plan**.
- **(a) POST commit statuses** (needs push + `repo` scope): **both accounts
  qualify — the active DylanDelliColli account suffices; no runtime account
  switching.** Every cycle comment on PRs 25/26 was posted by
  DylanDelliColli.
- **(b) Configure a required status check: NEITHER account can, on this
  repo, today.** Branch-protection and ruleset endpoints both return HTTP
  403 "Upgrade to GitHub Pro or make this repository public" — even with
  the admin token. The 2026-08-17 datapoint separated account-level admin
  (DDC-Heartwood only) from plan-level availability (currently neither).
  Making `adversarial-review` REQUIRED on market-intelligence needs an
  operator decision: make the repo public, upgrade the plan, or accept an
  advisory (non-blocking) status. **Marked for ARCHITECTURE.** Contrast:
  DylanDelliColli-org/abacus is public/org — rulesets answer 200
  (currently `[]`), so enforcement is available there; the gate is per-repo
  and onboarding docs should record it as an onboarding precondition.
- **Statuses API vs Checks API:** use the **commit-status API**
  (`POST /repos/{o}/{r}/statuses/{sha}`, context `adversarial-review`,
  pending→success/failure). Check runs can only be created by GitHub Apps —
  PATs are refused; every check run observed on PR 25's head is from the
  github-actions app. Required-check enforcement (where the plan allows it)
  matches on context string regardless of producing API, so the status
  route loses nothing. Quirk captured: the combined-status endpoint returns
  `state: "pending"` when ZERO statuses exist — a reader must not confuse
  "no statuses yet" with "pending check posted".

### 4. Precedent verification

**Confidence: high** (all artifacts read directly).

- **LAUNCHING.md contract**: reviewer = fresh context of a DIFFERENT
  lineage from the author, own detached pane, never headless from the
  authoring session, never split off the orchestrator pane. Choreography:
  pane split → `agent start --kind codex` → prompt BY FILE PATH → wait →
  `pane read --source recent-unwrapped --lines 400`. Recorded gotchas:
  fresh codex may stop at a hook-trust dialog (operator's decision, never
  key through); default pane read loses long output; resolve panes by agent
  name. Two deltas for this epic: step 1 splits a pane (the cluster now
  directs a dedicated workspace per reviewer — adoptable at no monitoring
  cost per §2), and step 4 uses the broken `--until idle` (§2). The
  two-comment amendment supersedes its "make no edits/print and stop"
  ground rule with exactly one permitted write: `gh pr comment` on the
  target PR.
- **PR 25/26 cycle records**: PR 25 carries 9 comments over 5 cycles; PR 26
  carries 4 over 3. Template shapes extracted — **reviewer verdict
  comment**: heading `## Adversarial review — cycle N` (or `# PR #<n> cycle
  <k> re-review`); numbered findings, each `**<severity>** — file:line
  cites + refutation reasoning` (severities blocker/concern/note); scoped
  re-reviews name the rework commit SHA; the clean form is "No findings. I
  could not refute rework commit `<sha>`." plus a `## Probes` section.
  **Adjudication comment**: `## Adjudication — cycle <k> (operator-ruled
  <date>)`; per finding accepted/rejected/rerouted with destination; clean
  form `**Verdict NOT REFUTED — accepted.**` plus verification summary. The
  cycle-2-refutes-incomplete-fix record is visible on PR 25 (cycles 2–4
  each REFUTED the prior rework). All comments authored by DylanDelliColli
  — consistent with §3(a).
- **Interim warm-worker practice (ab-co5): confirmed live** — workspace w33
  `market-brief-package-workers` operating now; PR 25's five same-branch
  rework cycles are the production trail.

### 5. Relatedness: ab-24o

**Not intersecting; no ordering constraint. Confidence: high.** ab-24o's
fix site is codex-side sandbox-profile construction; the abacus engine
contains no sandbox code (verified by grep). Zero file overlap with this
cluster. The only interaction is intensity: warm lanes multiply in-lane git
operations, so ab-24o's elevation friction gets more frequent once this
lands — an argument for scheduling it soon after, not for coupling.

### 6. Bundle candidates (all provisional — bead shapes are DECOMPOSITION's call)

- **Bundle 1 — outcome model + drain continuation + morning report (S5+S6,
  the wedge).** Footprint: src/lib.rs:82–136, src/main.rs:708/743/824–966,
  lib.rs unit fixtures, br_roundtrip.rs + a fake-shim drain test per
  tests/drain.rs. Predicted overlap: near-total — every new outcome class
  lands in the same enum, probe, and two match sites. One worker's bundle
  or strictly sequenced; parallel lanes would collide on every file.
- **Bundle 2 — warm-lane keepalive + reap-on-merge (S4).** Footprint:
  `should_reap_lane`, the reap block, `cmd_drain` loop state, merged
  detection reusing `parse_queue_state`/`observe_queue_state` or REST.
  Heavy overlap with Bundle 1 at `dispatch_cycle`'s settle path — sequence
  strictly after Bundle 1, same lane if possible.
- **Bundle 3 — reviewer launch + refutation-brief template (S1).**
  Footprint: a NEW module modeled on `dispatch_resolution` — dedicated
  workspace, codex agent, brief to a gitignored tmp path, prompt-by-file
  per LAUNCHING.md, `prompt --wait` per §2. One bundle; overlap with 1/2 is
  a single call site.
- **Bundle 4 — two-comment convention + status post/flip (S2+S3).**
  Footprint: comment-body builders beside `admission_red_park_body`,
  posting via the `comment_on_candidate` pattern, new commit-status
  helpers, adjudication-driven flip. Overlap with Bundle 3: the
  review-cycle data model (cycle number, verdict/adjudication shapes from
  §4) — worth a shared types seam. The reviewer posts its OWN verdict
  comment (ab-phr amendment), so the engine side here is only the
  adjudication comment + flip; verdict-posting instruction lands in Bundle
  3's brief template. The required-check ENFORCEMENT toggle stays out of
  all bundles pending the §3 operator decision.

### Assumptions that could invalidate the frame

1. **Codex-only monitoring evidence** — a claude-kind agent or a herdr
   update changing status vocabulary invalidates the
   `--until done`/`prompt --wait` guidance.
2. **"done" assumed terminal per turn** — if codex can transition
   done→idle later, wait semantics change; the default-states form is
   robust, `--until done` alone may not survive a "blocked" settle.
3. **S3's enforcement leg assumes configurable required checks** — false on
   the current production target; if the operator declines
   visibility/plan changes, S3 degrades to advisory-status-plus-engine-side
   gate and the success metric's "pending required check" clause needs
   rewording.
4. **BLOCKED signal assumes the deployed contract keeps the leading token
   "BLOCKED" in the LATEST comment** — verified today in market-brief
   AGENTS.md; contract rewording or a later non-BLOCKED comment silently
   reclassifies as Incomplete (supersede case is deliberate in the spec).
5. **Warm-lane re-derivation assumes deterministic names survive** —
   `sanitize_agent_name` truncates to 32 chars; colliding bead ids would
   alias agents. Not observed; DECOMPOSITION should note the class.
6. **N6's "no general concurrency" is under pressure if reap-on-merge
   forces long concurrent PR-watching** — the stateless re-derivation
   option is what keeps that pressure low.
7. **The br_roundtrip flake is assumed benign and pre-existing** — if
   ordering-dependent, this cluster's new tests in the same file amplify
   it; identify before the test budget is set.

---

*RESEARCH approved by operator 2026-08-18. Operator ruling at the gate:
S3 enforcement is advisory-plus-capability — the engine posts the status
everywhere; making it a REQUIRED check is repo configuration owned by
onboarding, available only where the repo's plan allows (abacus qualifies
today; market-intelligence does not until a visibility/plan change).*

## ARCHITECTURE

Produced inline by the orchestrator (recorded substitution: the default
gaudi producer's epic mode reviews an existing bead tree, which does not
exist until DECOMPOSITION; gaudi's interface-coherence concerns are applied
as the smell-risk checklist below). Locks the design for the approved frame
using the RESEARCH findings and the operator's gate rulings.

### A1 — Outcome model and the sweep/dispatch drain loop

**Two-layer classification, both stateless.** `BeadOutcome` (src/lib.rs)
stays a pure function of `br show` output and gains one variant: `Blocked` —
bead `in_progress` AND its highest-id comment's leading token is `BLOCKED`
(the deployed-contract signal; a later non-BLOCKED comment deliberately
supersedes). A new drain-level `LaneState` is re-derived per cycle from
three probes (`br show`, `herdr agent list`, `gh pr view --json
state,mergedAt` by deterministic branch `lane/<bead-id>`): `Authoring`,
`Blocked`, `AwaitingReview` (bead closed + PR open + no accepting
adjudication), `ReworkRequested` (adjudication comment requests rework),
`Merged`, `Stalled` (in_progress, agent settled, no BLOCKED comment).
**No state file** — every state is re-derivable after a crash, preserving
the engine's statelessness on a crash-prone host (RESEARCH §1).

**The drain becomes sweep-then-dispatch per iteration.** Sweep: re-derive
`LaneState` for every live lane (discovered from `herdr agent list` names
matching `sanitize_agent_name` of claimed/closed beads plus open `lane/*`
PRs, the `enumerate_candidates` pattern); act on transitions — launch a
reviewer for a newly `AwaitingReview` lane, flip status per a new
adjudication comment, redispatch rework, reap `Merged` lanes, park
`Blocked`/`Stalled` with per-class reporting. Dispatch: if no rework was
dispatched this iteration, claim the next ready bead as today. **At most one
ACTIVE worker turn at a time** — the engine still blocks on `prompt --wait`
— so N6 holds structurally: concurrency exists only as settled-warm lanes
awaiting adjudication. Rework outranks fresh dispatch. `cmd_run` keeps
single-cycle semantics with per-class exit reporting; only `cmd_drain`
loops the sweep. Drain never aborts on `Blocked`/`AwaitingReview`/
`Stalled`; it records, reports, continues, and exits when no ready beads
remain AND no lane transition is pending, summarizing lanes by class (the
morning report, S6).

### A2 — Review gate (engine-owned)

Review launch lives in the ENGINE (a new module, working name
`src/review.rs`), not in orchestrator choreography — zero-intervention
overnight requires it. On `AwaitingReview`: generate the refutation brief
from the bead (description + comments as the authority trail, the target
repo's AGENTS.md as the contract reference, per-bead refutation targets,
read-only ground rules with EXACTLY one permitted write — `gh pr comment`
on the target PR — and the required verdict line REFUTED / NOT REFUTED with
numbered findings plus a Probes section, template shapes per RESEARCH §4).
Write the brief to a gitignored tmp path in the target repo (onboarding
verifies the ignore rule). Launch: dedicated herdr workspace per reviewer,
`agent start --kind codex` (subscription OAuth by construction — N2 holds
structurally), prompt by file path, monitored with `prompt --wait`
(RESEARCH §2); reviewer pane is disposable after its verdict comment
posts. Reviewer cwd is the TARGET REPO MAIN CHECKOUT with `gh pr
diff`/`view` for the delta — the production-proven posture; a same-branch
worktree is impossible anyway (the warm lane holds the branch), and a
detached-HEAD worktree is deferred until main-checkout contention is
actually observed (MVP-first).

### A3 — Status lifecycle (advisory + onboarding-owned enforcement)

Commit-status API only, context `adversarial-review` (check runs are
GitHub-Apps-only, RESEARCH §3). Engine posts `pending` when a lane first
reaches `AwaitingReview`, and flips to `success` only when the sweep parses
an adjudication comment whose verdict accepts (`## Adjudication — cycle
<k>` + accepted/NOT-REFUTED verdict line — the two-comment convention: the
engine parses ONLY adjudication comments and the worker BLOCKED token,
never reviewer verdict bodies). REFUTED-with-rework keeps the status
`pending` through cycles; the engine never posts `failure` — a refuted PR
is being reworked, not dead; abandonment is a human PR-close. The engine
NEVER writes branch-protection or ruleset config: making
`adversarial-review` a required check is an onboarding act on repos whose
plan allows it (operator ruling at the RESEARCH gate), mirroring ADR
0003's posture that GitHub owns enforcement. Reader quirk honored: the
combined-status endpoint reports `pending` when zero statuses exist —
readers must distinguish absent from posted-pending (RESEARCH §3).

### A4 — Warm rework (S4)

On `ReworkRequested`: the engine prompts the EXISTING agent (name
re-derived via `sanitize_agent_name(bead_id)`) with a rework spec generated
from the adjudication comment (accepted findings + rework commit
expectations), on the same branch; the bead is reopened by the
adjudicating side per the existing convention. Recovery: if the warm agent
is gone (crash, operator close), recreate the lane on the SAME existing
`lane/<bead-id>` branch — the implementing bead must verify herdr's
worktree-create behavior against a pre-existing branch and fall back to
`git worktree add` + `herdr workspace create --cwd` if unsupported (bead-
level verification, not an open question). Reap moves from settle to:
`Merged` (always, force allowed as today) or operator abandon; `Blocked`
lanes reap only when clean (invert the existing force path via
`is_dirty_worktree_remove_error` — RESEARCH §1).

### Contracts locked (consumed by TEST-STRATEGY and DECOMPOSITION)

1. `BeadOutcome::Blocked` classification rule (highest-comment-id, leading
   token `BLOCKED`, supersede-able).
2. `LaneState` enum and its three-probe derivation; no persisted state.
3. Sweep-then-dispatch drain; one active worker turn; drain exit = no
   ready beads and no pending transitions; per-class summary.
4. Refutation-brief template (authority map, targets, one-write ground
   rule, verdict-line + Probes contract).
5. Adjudication-comment grammar as the ONLY machine-parsed review text;
   status flip rules (pending→success; never failure).
6. Status context `adversarial-review` via commit-status API; enforcement
   is onboarding-owned repo config.
7. Reviewer: ephemeral, dedicated workspace, main-checkout cwd, codex
   OAuth, `prompt --wait` monitoring.
8. Rework: same agent, same branch; deterministic-name recovery path.

### Smell and migration risks (the gaudi checklist)

- `dispatch_cycle` (already ~140 inline lines) absorbing sweep + review +
  rework is a god-function trajectory. Decision: Bundle 1's first act is
  extracting the lane lifecycle into its own module; `dispatch_cycle`
  becomes orchestration over named phases. The extraction is behavior-
  preserving and precedes new states.
- Comment-grammar coupling: three textual contracts (BLOCKED token,
  adjudication heading/verdict, brief template) now bind engine parsing to
  deployed repo contracts. Mitigation: single `src/review.rs` (or shared
  types seam) owns every grammar constant; AGENTS.md templates cite them;
  tests pin exact strings.
- Deterministic-name collision: `sanitize_agent_name` truncates at 32
  chars — DECOMPOSITION notes the collision class (RESEARCH assumption 5);
  no design change now.
- The sweep multiplies gh calls per iteration (per-lane PR probes).
  Mitigation: probe only lanes whose state can have changed (Blocked and
  Merged are absorbing until human action); acceptable at current lane
  counts; revisit only on observed rate pressure.

### Research assumptions disposition

Assumption 3 resolved by the S3 gate ruling (advisory + onboarding-owned
enforcement; success-metric wording adjusted at DECOMPOSITION to "verdict
comment + posted status" with required-enforcement where the repo supports
it). Assumption 6 resolved structurally (serial active turn + sweep).
Assumptions 1–2 (codex-only monitoring evidence) are carried as contract
7's caveat: any claude-kind lane re-verifies settle vocabulary first.
Assumptions 4, 5, 7 carried into DECOMPOSITION/TEST-STRATEGY as noted. No
research finding was silently promoted; no assumption invalidates the
frame.

---

*ARCHITECTURE approved by operator 2026-08-18.*

## TEST-STRATEGY

Produced by a columbo-type subagent (lifecycle-tests) 2026-08-18 at HEAD
`863a8a4`; integrated verbatim by the orchestrator. Grounding: the three
established harness patterns (lib.rs unit fixtures; tests/drain.rs
fake-shim; tests/br_roundtrip.rs real-workspace) plus the tests/land.rs
forbidden-call assertion style for gh coverage. New evidence gathered this
substage: live `br show --json` comment shape (integer ids,
second-granularity `created_at` — timestamp ties independently validate
contract 1's highest-comment-id rule) and the serde trap that a
commentless bead OMITS the `comments` field rather than emitting `[]`.

### 1. Story-by-test matrix (summary; full matrix as delivered)

**S5 (wedge):** 6 new unit classification tests in src/lib.rs's existing
test module — Blocked from highest-id BLOCKED comment (fixture shapes
matching today's live capture); supersede case including REVERSED array
order (max-id, never array position); absent-vs-empty comments field;
case-sensitive boundary-checked BLOCKED token from the shared grammar
constants; status-wins (closed + BLOCKED comment → Completed); plus two
existing tests widened in place without thinning
(`bead_status_classifies_the_three_worker_outcomes`,
`only_a_completed_outcome_reaps_the_lane`). LaneState derivation truth
table (~2 fns) in the new lane module. Integration: 2 fake-shim drain
tests (blocked-settle-continues with **exit 0 red against current HEAD —
the wedge's red-first proof**; awaiting-review-exits-cleanly) and 2
real-br tests (real `br comments add` blocked classification through the
real engine parse; superseded-BLOCKED → stalled — real because comment-id
assignment is precisely what is under test).

**S6:** 1 unit report-renderer test (per-class lines, bead ids, empty
classes omitted); in-context stdout assertions ride the S5 shim tests.

**S1:** 2 unit brief-contract tests in src/review.rs (modeled on the
house `dispatch_prompt_carries_bead_identity_and_protocol` style):
authority map, one-write ground rule with negative space (no git push /
br close / br update in the brief), verdict-line + Probes grammar;
deterministic gitignored tmp path. 1 fake-shim sweep test: exactly one
dedicated-workspace reviewer launch across two sweep iterations,
prompt-by-file-path, and a grep-negative that **`agent wait --until
idle` never appears** (Unknown A pinned as a regression tripwire).
Stated exclusion: the reviewer's own conduct is not suite-testable; the
enforcement surface is the brief text + the two-comment convention +
operator adjudication.

**S2:** 5 unit grammar tests in src/review.rs against fixtures captured
verbatim from the PR 25/26 production records: accepted adjudication;
rework-requesting adjudication; **reviewer verdict bodies are never
parsed as adjudications** (contract 5 negative space); latest
adjudication cycle wins; and the highest-leverage single test in the
cluster — the adjudication body builder round-trips through the parser,
making the grammar-coupling mitigation mechanical.

**S3:** 2 unit tests — status POST builder (context exactly
`adversarial-review`; a two-variant Pending/Success state enum makes
`failure` unrepresentable) and the combined-status reader distinguishing
absent from posted-pending (fixtures from live gh output; RESEARCH §3
quirk). 1–2 fake-shim tests: pending posted exactly once → no new POST on
rework adjudication → success only after acceptance; grep-negatives: no
`failure` state ever, no ruleset/branch-protection mutations (land.rs
forbidden-call style — contract 6).

**S4:** 1 unit reap-policy-by-state test (Merged always; Blocked only
clean; AwaitingReview/ReworkRequested/Stalled never). 4 fake-shim tests:
rework redispatches into the existing warm agent on the same branch with
ZERO worktree-create (the orientation tax S4 kills, as negative space);
rework outranks fresh dispatch within one iteration; vanished-agent
recovery recreates on the SAME `lane/<id>` branch; dirty Blocked lane
left standing with **no `--force`** — the deliberate inversion of
`abacus_run_warns_and_forces_removal_when_a_completed_lane_is_dirty`,
which stays verbatim for completed lanes (tripwire: two distinct
behaviors, never folded into one parameterized test).

**Cross-cutting regression net:** the extraction is gated by the existing
~89 tests surviving by name AND assertion body; five named settle-path
survivors are listed as must-preserve.

### 2. Seam placement

Real-br integration capped at exactly 2 tests (comment-id assignment and
JSON shape are the seam); all nine sweep/dispatch/rework/status flows on
fake shims with call-log and forbidden-call assertions; **gh interactions
explicitly NOT integration-tested against real GitHub** — network, auth
identity split, and the plan-gated 403 make it unrepeatable; the boundary
is fixture-tested parsers/builders plus fake-gh shims, with residual
gh-CLI-drift risk accepted on the same terms the land suite already
accepts (every gh call propagates error text through `capture`).

### 3. Budget

Measured baseline at `863a8a4`: 9.48–12.88s over 8 warm runs (RESEARCH's
7.86s did not reproduce under today's concurrent-agent load; budgeting
uses the worst observed number). Remaining: 30 − 12.9 ≈ 17.1s. Additions:
17 unit ≈ <0.05s; 9 fake-shim ≈ 1.2s; 2 real-br ≈ 0.9s — ≤2.2s
pessimistic serial, ~1s effective (thread-pool width-domination).
Projected post-cluster suite **~13–15s worst case; the budget holds with
≥15s headroom; no cuts required.**

### 4. Flake hunt — identified

Reproduced 1/18 executions:
`dispatch_protocol_pushes_opens_pr_then_closes_without_lane_tracker_changes`
(tests/br_roundtrip.rs:1511), panicking in the `br()` helper on br's own
validation: `updated_at: cannot be before created_at`. Mechanism: a
br-side wall-clock race on quick create-then-write sequences
(non-monotonic clock reads on this WSL2 host or dual clock sources in
br) — a CLASS, not a single test; it fired in isolation, refining
RESEARCH's ordering hypothesis away. Benign to correctness (loud,
attributed, workspace-isolated) but each new real-br create-then-write
adds an exposure window — a deliberate reason real-br additions are
capped at 2. Recommended hardening (small DECOMPOSITION bead): one
~100ms retry in the test `br()` helper matched to this exact validation
message; quenches the existing 23 tests' exposure and cannot mask real
failures. RESEARCH assumption 7 disposed: real, pre-existing, benign,
not ordering-dependent. (Jot filed with full repro.)

### 5. Close

**28 new tests — 17 unit, 11 integration; zero new files under tests/;
~1–2.2s added wall.** Two items exit the substage: (1) OPERATOR QUESTION
— the exit-code contract per settle class: proposal is `abacus drain`
exits 0 whenever the loop drains without infrastructure error regardless
of class mix (the morning report is the signal), and `abacus run` keeps
0 = Completed, gains one distinct nonzero (3) for classified
non-completed settles so wrappers distinguish parked-by-design from
engine failure (1). The two drain SHIM tests assume drain-exit-0 and
flip if ruled otherwise. (2) DECOMPOSITION note — the post-adjudication
pre-merge state has no LaneState name; simplest is
AwaitingReview-with-flipped-status until Merged, alternative is a
distinct `Landing` state; plus the `sanitize_agent_name` 32-char
collision note rides the warm-rework bead ([no-test], not constructible
with realistic ids).

---

*TEST-STRATEGY approved by operator 2026-08-18. Exit-code contract ruled
as proposed at the gate (later amended by spec-validation finding 2 — see
RECORD).*

## RECORD

An ADR is warranted and produced: **docs/adr/0005-lane-lifecycle-v2.md**
(decisions D1–D8 with the test contract), placed under `docs/adr/` so the
design-document review gate fires. Review evidence, both passes run as
fresh Codex contexts in dedicated panes of workspace adr5-review:

- **Bloat review** (PLANNING-adr5-bloat-review.md, one pass, four cuts,
  anchor explicitly the abacus north star per the skills-y13 lesson):
  operator-disposed — cuts 1 (defer the review cluster), 2 (drop
  extraction-first), and 4 (defer run exit 3) rejected with recorded
  grounds; cut 3 accepted as a trim (runtime projection removed, durable
  test-contract content retained). The reviewer's revive-when condition
  for cut 1 (adjudication inside the agent team, check-flip authority to
  an agent reviewer for overnight runs) is recorded in the ADR status
  block as a possible future evolution requiring a fresh operator ruling.
- **Spec validation** (PLANNING-adr5-spec-review.md, five findings, all
  applied): (1) BLOCKER — stateless probes could not distinguish consumed
  from unconsumed review/rework cycles; resolved by durable-fact cycle
  bookkeeping: the adjudication grammar gains the adjudicated-head SHA,
  verdict comments are recognized by heading only for existence/cycle
  counting, reviewer agent names are deterministic per bead and cycle,
  and ReworkRequested holds only while the branch head equals the
  adjudicated head. (2) HIGH — the Completed/AwaitingReview exit-code
  overlap; resolved by lane-state ownership of exit codes, amending the
  TEST-STRATEGY ruling so AwaitingReview is run's nominal exit-0 outcome
  (ratification at this gate). (3) HIGH — per-finding adjudication
  dispositions restored to D4 (preservation of the ab-phr two-comment
  contract). (4)–(5) MEDIUM — herdr and GitHub context summaries narrowed
  to exactly the measured boundaries.

Both reviews' findings are resolved in the committed ADR; the operator's
RECORD-gate approval covers the ADR as amended plus the D6 ratification.

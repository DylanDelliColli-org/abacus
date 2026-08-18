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

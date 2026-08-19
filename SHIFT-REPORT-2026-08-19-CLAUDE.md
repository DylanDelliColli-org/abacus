```doc-meta
role: handoff
lifecycle: inflight
```

# Shift report — 2026-08-19 (orchestrator handoff, abacus, pre-execution)

Supersedes `SHIFT-REPORT-2026-08-15-CLAUDE.md`, deleted from the tree in
this report's commit; git history is the archive.

## 1. Identity and snapshot boundary

Repository `~/dev-environment/abacus` (GitHub
`DylanDelliColli-org/abacus`, public, org-owned), branch `main`, remote
attached. Outgoing: the Claude orchestrator (workspace `w1M`) that ran
2026-08-17 through 2026-08-19 — the market-brief onboarding, the
lifecycle-v2 planning run, and two jot-review passes. Incoming: the
orchestrator that executes the lifecycle-v2 epic. **Pre-report base:
`908de42`** (equal to `origin/main` when probed ~11:35 -04:00
2026-08-19; the working tree at authoring carries only `.beads` changes
made during this report's own probes — the go4.1→ab-66o dep and two
comments — which ride this report's commit). Resolve this report's own
commit after intake:
`git log --oneline -1 -- SHIFT-REPORT-2026-08-19-CLAUDE.md`.

## 2. Read-first authority map

- **`NORTH-STAR.md`** — unchanged since the 2026-08-15 amendment.
- **`docs/adr/0005-lane-lifecycle-v2.md`** — **accepted 2026-08-18; the
  execution contract's authority.** D1–D8: two-layer stateless outcome
  model, sweep-then-dispatch drain, engine-owned review gate,
  adjudication-only parsing with the adjudicated-head SHA, warm lanes,
  ratified exit codes, extraction-first, single-owner grammars. Its
  status block carries the full two-review trail (bloat cuts disposed;
  five spec findings applied).
- **Epic `ab-lifecycle-v2-go4`** and children `.1`–`.7` — the execution
  contract itself; every child passed the Fresh Agent Test under a cold
  freshness review. The epic description carries execution order and
  Success Criteria.
- **`docs/adr/0004-foreign-repo-onboarding.md`** — governs the
  market-brief relationship. **Known record defect: its status still
  reads "proposed" although its epic executed to completion and closed
  2026-08-17** — see §9 corrections; flipping it is a two-line
  housekeeping edit awaiting an operator nod.
- **ADRs 0001–0003**, **AGENTS.md**, **CONSTRAINTS.md**,
  **`docs/INDEX.md`** — standing authorities, unchanged.
- Work state: `br` 0.3.2 in `.beads/`. **The installed `abacus` binary
  predates the lifecycle epic (no engine source has changed since PR 25;
  suite verified green at authoring) and is the binary that will RUN the
  drain — see hazard 3.**
- Machine config: `~/.claude/CLAUDE.md` is still bd-era; the approved
  br rewrite draft now lives durably at **`~/.claude/CLAUDE.md.br-draft`**
  (regenerated this day after the original was lost with an expired
  session scratchpad — §9).

## 3. Objective and success condition

Execute the lane-lifecycle-v2 epic: seven sequenced children that give
the engine an outcome model (drain survives blocked/awaiting lanes), an
engine-owned adversarial review gate with durable two-comment verdicts
and an advisory `adversarial-review` commit status, and warm author
lanes reaped on merge. Success is the epic's Success Criteria (in its
description): an unattended drain of ≥3 beads ends fully classified,
every PR reviewed-with-status, rework dispatched warm, zero manual pane
interventions. The planning flow is additionally judged by its own bar,
recorded post-drain as epic notes: zero lanes stopping for missing
scope. Outside scope: auto-merge (the `ab-automerge-2b2` epic still
waits on the operator's `.1` queue configuration and `.8` live
validation); anything in ADR 0005's six non-goals (no CI-authoritative
review, no API-key reviewer auth, no warm reviewers, no automated
adjudication, no BLOCKED-resume machinery, no general parallelism or
bundling).

## 4. Direction changes and settled decisions (apply, do not re-litigate)

1. **ADR 0005 accepted (2026-08-18)** through the full gauntlet: four
   bloat cuts operator-disposed (cluster-deferral, extraction-drop, and
   exit-code-drop rejected with recorded grounds; test-contract trimmed),
   five spec findings applied (durable cycle bookkeeping via the
   adjudicated-head SHA; lane-state ownership of exit codes; per-finding
   adjudication dispositions restored; two measured-boundary
   corrections). The D6 amendment is ratified: `AwaitingReview` is
   `run`'s nominal exit-0 outcome; 3 = Blocked/Stalled; 1 = failure.
2. **The three input beads (`ab-phr`, `ab-co5`,
   `ab-blocked-lane-outcome-6bs`) are CLOSED as absorbed** — specs in
   ADR 0005, scope in the children. Do not reopen; their comment trails
   remain citable history.
3. **Monitoring truth (measured, supersedes ab-co5's original claim):**
   `herdr agent wait --until idle` never fires for settled codex agents
   in ANY topology (codex settles at `done`); `prompt --wait` works
   everywhere; `--until done` verified in a dedicated root pane only.
   bb-skills fixed LAUNCHING.md accordingly on 2026-08-19.
4. **Enforcement is plan-gated per repo**: required checks are
   configurable on abacus (public/org) and NOT on market-intelligence
   (private, free plan) even for the admin account. The engine posts
   advisory statuses everywhere; making them required is an onboarding
   act (RECORD-gate ruling).
5. **Reviewer role-card refinements (operator-ruled 2026-08-18, curated
   2026-08-19)** are appended as a comment on `go4.4` and must be
   encoded in the brief template: execution bar (a blocker requires an
   executed failure or byte-level demonstration), threat model per
   finding, convergence pressure after cycle two.
6. **Adjudication grammar carries `Adjudicated head: <sha>`** —
   adopted in market-brief production effective PR 27 cycle 2; the
   engine's parser (go4.5) targets the fixed grammar only.
7. Standing and unchanged: SABLE machinery is retire-not-fix; jot
   curation is operator-invoked only; reviews run as fresh codex panes
   of the other lineage with the governing north star named explicitly
   (bb-skills `skills-y13`); no `--admin` merges ever.

Unresolved operator choices, parked: `ab-automerge-2b2.1`/`.8`
(operator seats); applying `~/.claude/CLAUDE.md.br-draft` (direction
approved 2026-08-17, concrete text awaiting operator review); ADR 0004
status flip; in the market-brief seat — GEMINI.md keep-and-trim, the
CI-red governance ruling, SABLE-exclusive bead deletion post
merge-path-successor ruling.

## 5. Durable work state

**Landed (pushed at or before `908de42`):** ADR 0005 + epic
`ab-lifecycle-v2-go4` with children `.1`–`.7` (planning records deleted
per ADR 0001 — git history holds
`PLANNING-ab-lifecycle-v2-go4.md` and both review files); the
market-brief onboarding epic `ab-mbp-onboard-hb0` CLOSED 2026-08-17
(migration `5d4394c`, AGENTS.md `07d57a7`, branch flip, PRs 23/24 both
since merged); `ab-66o` filed by the market-brief cockpit session with
this session's sequencing dep and discriminating-check comment; the
`go4.4` reviewer-refinement comments; jot queue drained to zero across
two operator-invoked reviews (promotions: the preflight evidence on
`ab-init-plan-5ka`, `skills-y13` in bb-skills, LAUNCHING.md fix,
reviewer refinements).

**In flight:** nothing in abacus — no lanes, no claimed beads, no open
PRs (probed: `gh pr list` returned `[]`). Three idle planning subagents
from this session (research/tests/freshness producers) are spent and
disposable.

**Uncommitted:** at authoring, only the probe-time `.beads` changes
named in §1, which this report's commit carries. External durable
artifact: `~/.claude/CLAUDE.md.br-draft` (untracked machine config,
operator-review pending).

**Planned (open beads, verified this hour):** the lifecycle chain
`ab-66o` → `.1` → `.2` → `.3` (also needs `.7`) → `.4` → `.5` → `.6`;
independents `.7`, `ab-24o` (sandbox writable-roots, another session's
filing, judged non-intersecting by RESEARCH); operator-seat
`ab-automerge-2b2.1`/`.8`; planning triggers `ab-init-plan-5ka` (fully
evidence-loaded, still behind `ab-virgin-bootstrap-jjq`),
`ab-testvalues-consumption-d01`, `ab-braindump-phase-wow`,
`ab-bundling-plan-4mu`.

**Parked:** jot queue empty (probed: "no pending notes").

## 6. Ownership and boundaries

- **market-brief-package is the `market-brief-package-d2` session's
  cockpit** (workspace `w3`, workers in `w3B`
  `market-brief-package-workers` — 4 panes, working at probe time).
  Its lanes (PR 27, mb-5jze at last coordination), tracker writes, and
  the interim warm-worker practice belong there. Abacus sessions do not
  run `abacus drain` against that repo (hazard 4) and do not touch
  `w3B` — `ab-66o` exists because parked warm workers there died twice.
- **bb-skills** is its own session's seat (`w2W`); its store remains
  schema-5 (`br-0.1.45 --db` only, until `skills-3vx`).
- Operator seats: `ab-automerge-2b2.1`/`.8`, the CLAUDE.md draft
  apply, ADR 0004 flip, and all market-brief governance rulings.
- The incoming abacus orchestrator owns: dispatching the lifecycle
  chain, PR shepherding to the operator, and the post-drain
  planning-bar note on the epic.

## 7. Hazards, holds, and negative instructions

1. **Do not merge PRs in any repo.** Operator's gate. Release: an
   explicit grant, or (abacus only) the auto-merge queue going live via
   `ab-automerge-2b2.1`+`.8` — at which point `--admin` remains
   forbidden always (ADR 0003).
2. **Do not dispatch `.1` (or any wedge child) before `ab-66o`
   closes.** The dep now enforces it. Rationale on the bead: identical
   reap-block footprint; extracting before fixing bakes the defect into
   the new module. Note for the ab-66o lane: the engine removes by
   recorded workspace id, so if the repro kills a foreign workspace the
   defect is likely herdr-side — the bead's red-first repro
   discriminates.
3. **The running engine is the installed binary, not the source tree.**
   The lifecycle lanes modify the engine's own source; the drain
   executing them runs `~/.cargo/bin/abacus` built from pre-epic
   source. After each engine PR merges, `cargo install --path .` before
   relying on the new behavior. Until `.3` lands AND is installed, a
   BLOCKED or incomplete lane still ABORTS a drain (current behavior) —
   babysit accordingly; that annoyance is precisely what the wedge
   removes.
4. **`abacus drain` remains forbidden against market-brief-package**
   until that seat's triage rulings land (their `br ready` still
   carries non-dispatchable pivot-stale items). `abacus run` on an
   operator-selected bead is their session's call, not this one's.
5. **bb-skills tracker: never write with br 0.3.2** (schema 5). Use
   `~/.local/bin/br-0.1.45 --db`. Release: `skills-3vx` closed.
6. Standing: commit `.beads` at natural points, pull
   `--rebase --autostash`; review dispatches name the governing north
   star explicitly and carry the no-bead instruction; session
   scratchpads are EPHEMERAL — anything an operator will review later
   gets a durable home immediately (learned the hard way, §9).

## 8. Incoming boot sequence

```sh
cat NORTH-STAR.md
br show ab-lifecycle-v2-go4        # epic: order, success criteria
br show ab-66o                     # first lane's scope + sequencing comment
br ready                           # expect: ab-66o, .7, automerge-2b2.1, ab-24o, planning triggers
git -C ~/dev-environment/abacus status --short && git log --oneline -3
cargo test 2>&1 | grep 'test result'   # expect 89 passed / 0 failed across ten lines
```

**First consequential act:** dispatch the first lane —
`abacus run ~/dev-environment/abacus` — which selects `ab-66o` (P1,
ready, wins `select_bead`; `.7` is P2). Expect possible manual
shepherding per hazard 3: the current engine treats any non-closed
settle as a failure with exit 1.

## 9. Verification ledger and known defects

| Claim | Probe | Observed (authoring, 2026-08-19 ~11:30–11:50 -04:00) | Incoming action |
|---|---|---|---|
| main equals origin/main at pre-report base | `git rev-parse HEAD origin/main` | both `908de42`; tree then carried only this report's probe-time `.beads` edits | Recompute |
| Suite green | `cargo test`, all result lines | 89 passed / 0 failed / 0 ignored over ten suites; br_roundtrip 11.0s, full wall ~17s under load | Recompute before first lane |
| No open abacus PRs | `gh pr list` (abacus) | `[]` | Recompute |
| Open beads | `br list --status open --status in_progress` | exactly the 19 items summarized in §5 (incl. epic rows) | Recompute |
| Lifecycle ready front | `br ready` | ab-66o and `.7` are the epic-adjacent ready items; `.1` correctly blocked by ab-66o | Recompute |
| Jot queue empty | `jot list` | "no pending notes" | Trust; capture continues |
| PR 24 (market-brief doc PR) merged | `gh api .../pulls/24` | `state: closed, merged: true` | Trust as durable |
| Market-brief warm workers alive | `herdr workspace list` | `w3B market-brief-package-workers`, 4 panes, working | Recompute — volatile, other session's property |
| CLAUDE.md draft durable | `ls ~/.claude/CLAUDE.md.br-draft` | present, regenerated this day | Trust; operator reviews before apply |
| br_roundtrip flake | TEST-STRATEGY 2026-08-18 record | identified: clock-race in br validation, ~few %/run; fix specced in `.7` | Trust the identification; `.7` carries the fix |

**Corrections at authoring (kept visible, per the failed-handoff rule):**

- **The global-CLAUDE.md rewrite draft was lost.** The 2026-08-17
  session stored the operator-review artifact only in its session
  scratchpad and my reports cited that path; the scratchpad expired
  with the session. Regenerated 2026-08-19 from the verified command
  mapping to `~/.claude/CLAUDE.md.br-draft` (durable). Lesson, now
  hazard 6: a session scratchpad is not a durable home for anything an
  operator will act on later.
- **ADR 0004 still says "proposed."** The onboarding it records
  executed to completion and its epic closed 2026-08-17; nobody flipped
  the status. Not silently fixed here because ADR status changes ride
  operator acceptance; flip on the operator's nod.
- The 2026-08-15 report's "next engine PR" reinstall note remained
  unexercised — no engine source has changed since; the note graduates
  into hazard 3, which will bind on every lifecycle lane.

## 10. Closeout pointer

This report is the index; ADR 0005, the epic and its children's
descriptions (which deliberately carry full context), `ab-66o` with its
sequencing comment, and the closed input beads' comment trails are the
cargo. On intake mismatch, stop only the affected action and correct
this file durably. The outgoing session commits this report (deleting
its predecessor), pushes, and ends.

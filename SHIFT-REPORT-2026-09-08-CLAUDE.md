# Shift report — abacus orchestrator seat, 2026-09-08

## 1. Identity and snapshot boundary

Repository `~/dev-environment/abacus`, branch `main`. Outgoing: the
Claude orchestrator session that ran the 2026-08-27→09-01 arc (planning
epic `ab-ljn`, the four-PR execution wave, the shadow trial, the
model-selection experiment, ADRs 0006/0007). Incoming: any orchestrator
session under `/abacus-execute`. Pre-report base SHA: `7aedf4c` (do not
read this as current HEAD — committing this report moves HEAD).
Observations in this report taken 2026-09-08, ~15:00Z, after a 7-day idle
gap; every mutable claim carries its probe in §9 and must be recomputed on
intake.

## 2. Read-first authority map

- **Evaluator stack, seats, model/effort config:** `docs/adr/0007-*.md`
  (proposed; through bloat + spec review, operator-disposed). Its D6 makes
  `docs/compatibility/2026-08-31-reviewer-model-selection-experiment.md`
  the binding evidence record — engage its data, never argue past it.
- **Two-mode review operation (engine vs manual), heading grammar,
  adjudication grammar, convergence controls:** `docs/adr/0006-*.md` +
  `.claude/skills/abacus-execute/SKILL.md` (the skill carries the manual
  procedure and hazards; ADR 0006 binds what survives rewrites).
- **Lane lifecycle, drain semantics, closure-vs-acceptance:** ADR 0005 and
  `docs/lifecycle.md`.
- **Work state:** `br` only. `br ready` is the plan.
- **Discovery:** the jot queue (20 pending — operator-invoked `/jot-review`
  only; never curate it yourself).

## 3. Objective and success condition

Standing objective (NORTH-STAR): a backlog drains overnight, zero operator
interventions, PRs review-gated. The current arc's deliverable: the
three-seat evaluator stack (ADR 0007) made real in the shipped contracts.
Success for the next session: `ab-ljn.5` and `ab-ljn.6` landed through the
full review flow, at which point epic `ab-ljn` is close-eligible and the
stack is operational end to end.

## 4. Direction changes and settled decisions (apply, do not re-litigate)

Chronological; each is operator-ruled and durably recorded.

1. **Manual mode is a permanent first-class fallback; engine mode is the
   destination** → ADR 0006 D5, with the comment-stream equivalence
   invariant. Supersedes the old skill §1 absolute ban framing.
2. **Strict heading-first verdict grammar** (prose-prefaced bodies never
   count; relays carry attribution after the body; legacy prefaced relays
   are recovered by re-post) → operator design ruling in PR 48's cycle-2
   adjudication; ratified in the merged parser. Supersedes `ab-cye`'s
   original tolerant-scan recommendation.
3. **Three-seat stack** (correctness gating / simplicity advisory /
   test-quality advisory) → ADR 0007 D1, exercising ADR 0006 D8's
   promotion path on the shadow-trial evidence (`ab-ljn.3` closing
   comment).
4. **Methodology sections are canonical brief content and a precondition
   of model selection** → ADR 0007 D2. The experiment's central finding:
   an evidence bar does not elicit the executed method.
5. **All seats run `gpt-5.6-sol` at `medium` with methodology briefs;
   Luna disqualified at both tested efforts; missed-blocker tripwire** →
   ADR 0007 D4/D5.
6. **Scope carve** (test-suite design → test seat; guard-neutralization
   stays with correctness; simplicity's tests shrink to incidental) →
   ADR 0007 D3.

Unresolved operator decisions (do not proceed past them):

- **Recovery-gating class, third instance.** The deferred-status drain
  crash (jot `20260831T200724Z`, verified live: `classify_bead_status`
  has no `deferred` arm, `src/lib.rs:144-151`; an ordinary concurrent
  `br defer` kills a drain with exit 1) is the third live instance of the
  class reworked in PR 46 cycles 2-3. The convergence contract requires an
  operator DESIGN ruling on the class, not another point patch. Surface it
  before any bead is filed from that jot.
- **Jot queue (20 pending)** awaits operator-invoked `/jot-review`.

## 5. Durable work state

**Landed** (all pushed; `main` == `origin/main` at the pre-report base):
PRs 45–50 merged — `ab-xuz` (amended correctness contract), `ab-omt` +
`ab-645` (default-branch and recovery/naming fixes, 3 review cycles),
`ab-ljn.1` (two-mode skill + simplicity template), `ab-stk` + `ab-cye`
(COLLABORATOR gate + strict heading, 4 cycles incl. one design ruling),
`ab-ljn.2` + `ab-d94` (D3 pinning test + roundtrip determinism), `ab-le5`
(simplicity follow-ups — first engine-launched review under the amended
contract). ADR 0006, ADR 0007, the observation record, `docs/lifecycle.md`
all committed. Installed `abacus` binary built from merged main
(post-PR-50) — current.

**In flight:** nothing. No open PRs, no live worker/reviewer agents, no
review worktrees (all reaped this session; sole surviving worktree is the
main checkout — probe §9).

**Uncommitted:** `.claude/skills/abacus-plan/SKILL.md`, a 5-line
operator-owned edit predating 2026-08-27. Do not commit, revert, or build
on it; it is the operator's. (`.beads/issues.jsonl` dirty state is this
report's own tracker activity and rides the report commit.)

**Planned (the ready front, in priority order):** `ab-ljn.5` (P1 —
test-quality template, heading registry, adjudication paragraph, D3 carve;
docs-only, grep-anchored AC) and `ab-ljn.6` (P2 — engine
`REFUTATION_BRIEF_TEMPLATE` gains the D2 methodology; red-first, D8
single-owner discipline). Independent footprints; parallel lanes are safe.
Also ready: three seat:operator planning beads (`ab-testvalues-consumption-d01`,
`ab-braindump-phase-wow`, `ab-bundling-plan-4mu` — the last now carries
two hard field constraints in the jot queue: bundled lanes are invisible
to the engine on both the merge-group and dispatch sides) and
`ab-virgin-bootstrap-jjq` (second-machine walkthrough). None of the four
is engine-dispatchable work — defer them before any `abacus drain` and
undefer after, or the engine will dispatch them as lanes.

**Parked:** `ab-init-plan-5ka` (blocked on virgin-bootstrap; carries
routed RESEARCH inputs and a citation correction in comments). `ab-stk`'s
sibling concern — reviewer identity: reviewers post under whatever `gh`
account is active; `DylanDelliColli` is required for adjudications and is
the currently active account (probe §9).

## 6. Ownership and boundaries

Single-operator repo. The orchestrator seat owns dispatch, review
launches, adjudication *posting* (rulings are the operator's), tracker
hygiene, and pushes. Operator-only: repo configuration, merge grants
(the 2026-08-28 session grant DOES NOT carry forward — re-request),
jot curation, design rulings, the `abacus-plan/SKILL.md` dirty edit.
No live messages pending to any agent; all experiment/reviewer contexts
are dead and reaped.

## 7. Hazards, holds, and negative instructions

- **Never let a drain run while operator-paced beads are ready.** The
  engine dispatches anything in `br ready`. Defer/undefer around drain
  windows (this bit twice on 08-28/31; the spurious-lane cleanup is
  expensive).
- **A concurrent `br defer` can kill a running drain** (the unruled
  third-instance defect above). Until ruled and fixed: do not defer beads
  while a drain is executing.
- **Bundled lanes (multi-bead branches) are engine-invisible** — no status
  reconciliation, no merge-group posting, spurious "recovery" lanes for
  their in-progress beads. Until `ab-bundling-plan-4mu` lands: bundle only
  with manual status flips budgeted, or use single-bead lanes.
- **Paste race is live on the manual path:** `--wait --until working` can
  return with the prompt unsent (observed 09-01 on the ADR 0007 bloat
  reviewer). Verify engagement by pane context-% or output; recover with
  `herdr agent send-keys <agent> Enter`. Trust deliverable signals (files,
  PR comments, bead status), never agent-state waits (`done` race and
  idle-exclusion trap both observed and documented in the jot queue).
- **zsh does not word-split `set -- $a`** — use colon-split loop variables
  for herdr batch commands, or every loop silently garbles flags.
- **Do not edit the §4 adjudication grammar block** in the skill or the
  engine constants without treating ADR 0006's equivalence invariant as
  the contract: byte-compatible artifacts across modes.
- **The evidence record is load-bearing** (ADR 0007 D6): proposals to
  change seats/config engage its data or supersede it with measurement.

## 8. Incoming boot sequence

1. `cat ~/dev-environment/abacus/SHIFT-REPORT-2026-09-08-CLAUDE.md` (this
   file), then `br ready` and `br blocked` — the plan and its gates.
2. `git -C ~/dev-environment/abacus fetch origin && git -C ~/dev-environment/abacus status`
   — expect clean-except the operator's `abacus-plan/SKILL.md` edit and
   up-to-date with origin.
3. `gh pr list` and `herdr workspace list` — expect zero open PRs and no
   abacus worker/review workspaces beyond the session's own.
4. `gh api user --jq .login` — expect `DylanDelliColli` (adjudication
   identity); if not, coordinate with the operator before any adjudication.
5. Read `docs/adr/0007-*.md` in full (it is short and every current
   decision lives there), then ADR 0006 §D5 and the skill's manual
   procedure if operating manually.

First consequential act: dispatch `ab-ljn.5` and `ab-ljn.6` as two
parallel Sol-medium lanes per the skill's manual launch procedure (or
single-bead engine dispatch — both are engine-visible lane/<id> shapes),
with methodology briefs per ADR 0007 D2. Their PRs are the new stack's
first production review cycle: launch all three evaluators.

## 9. Verification ledger

All observations 2026-09-08 ~15:00Z unless noted. Recompute every row on
intake.

| Claim | Evidence / probe | Observed | Incoming action |
|---|---|---|---|
| main == origin/main at pre-report base | `git fetch && git rev-parse HEAD origin/main` | both `7aedf4c` | rerun (report commit moves HEAD; verify push instead) |
| Full suite green on main | `cargo test --quiet` | 0 failures across all targets | trust; rerun before code work |
| Ready front is exactly 6 beads | `br ready` | ab-ljn.5, ab-ljn.6 + 4 operator-paced | rerun; defer the 4 before any drain |
| Blocked set | `br blocked` | 4 rows (epic ab-ljn by .5/.6; init-plan chain) | rerun |
| Tracker lint clean | `br lint` | "No template warnings (8 issues checked)" | trust |
| No open PRs | `gh pr list` | empty | rerun |
| No stray worktrees/lanes | `git worktree list` | 1 (main checkout only; 3 stale merged-lane worktrees reaped during authoring) | rerun |
| Active gh identity | `gh api user` | `DylanDelliColli` (implied by 08-31 flips; NOT re-probed at authoring — verify) | **rerun before adjudicating** |
| Jot queue depth | `jot list` | 20 pending | operator-invoked review only |
| Deferred-status drain crash live | `sed -n '144,151p' src/lib.rs` | no `deferred` arm at pre-report base | reconfirm before/while ruling |
| Installed binary current | built post-PR-50 merge 08-31 | not re-verified today | `cargo install --path .` before first drain if in doubt |

Known report defect, kept visible: the gh-identity row was inferred from
08-31 activity, not probed at authoring time — the boot sequence makes it
step 4 for exactly that reason.

## 10. Closeout pointer

This report is committed at the repo root (path-scoped history:
`git log --oneline -1 -- SHIFT-REPORT-2026-09-08-CLAUDE.md`) and pushed;
the tracker rides the same push. The outgoing session ends after this
commit. Next session: boot sequence above, then `ab-ljn.5`/`.6`.

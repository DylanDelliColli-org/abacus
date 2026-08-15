```doc-meta
role: handoff
lifecycle: inflight
```

# Shift report — 2026-08-15 (orchestrator handoff, abacus)

Supersedes `SHIFT-REPORT-2026-08-14-CLAUDE.md`, deleted from the tree
in this report's commit; git history is the archive.

## 1. Identity and snapshot boundary

Repository `~/dev-environment/abacus` (GitHub:
`DylanDelliColli-org/abacus` — transferred from the personal account
this day), branch `main`, remote attached and equal at authoring.
Outgoing: the Claude orchestrator (workspace `w1M`) that ran all of
2026-08-15. Incoming: a fresh orchestrator — **possibly on a machine
that has never known abacus** (the operator is switching computers; §8
branches on that). **Pre-report base:** `d2233bc` (equal to
`origin/main` at authoring, ~15:15 -04:00). Resolve this report's own
commit after intake:
`git log --oneline -1 -- SHIFT-REPORT-2026-08-15-CLAUDE.md`.

## 2. Read-first authority map

- **`NORTH-STAR.md`** — amended this day by operator revise mode with
  operator-approved wording: overnight merging of pending PRs enters
  the success condition for opted-in repositories; non-goal 3
  qualified. Amendment log carries the prior blob.
- **`docs/adr/0003-pr-validation-and-auto-merge.md`** (accepted) — the
  auto-merge design: GitHub's merge queue owns ordering, merge_group
  validation, and the merge; abacus does admission → enqueue →
  exception watch. Status block carries the full three-reviewer trail
  including the mid-review pivot that deleted the engine-owned merge
  loop. **ADR 0001** (planning flow) and **ADR 0002** (shared store,
  close-last protocol) stand.
- **`AGENTS.md`** — worker contract; gained the land-mode exception
  (workers still never merge; the engine enqueues) in PR 21.
- **`CONSTRAINTS.md`**, **`docs/INDEX.md`** — unchanged authorities.
- **Work state:** `br` 0.3.2 in `.beads/`. **The installed `abacus`
  binary was built from PR-24-era source; PR 25 changed tests only, so
  the binary is functionally current at the pre-report base.**
- The knowledge/skills repo is now **`~/dev-environment/bb-skills`**
  (operator renamed it from `~/dev-environment/skills` this day). Its
  own AGENTS.md governs there; its `br` store uses the `skills-`
  prefix and is **schema 5** — see hazards.

## 3. Objective and success condition

North star as amended: an overnight multi-bead drain across two
repositories, zero interventions, and on opted-in repositories the
engine merges pending PRs serially with parked-PR evidence as the
morning exceptions. **Position: the engine side is built and merged
(run / drain / land all exist and are tested at 89/0/0) but auto-merge
has never run live** — it waits on the operator's queue configuration
(`ab-automerge-2b2.1`) and the live validation (`.8`). The full loop
minus the queue was demonstrated this day by interim manual landing:
eight lanes dispatched, eight PRs (18–25) validated on composition and
merged, zero lanes stopped for missing scope.

## 4. Direction changes and settled decisions (apply, do not re-litigate)

1. **Auto-merge entered the thesis and got its ADR.** Full-tier
   planning run (`ab-automerge-2b2`), north-star amendment, ADR 0003
   accepted. Mid-review, the operator supplied the org fact and
   adopted the bloat reviewer's escalation: **the engine never merges;
   GitHub's queue does** — the engine-owned merge loop was designed
   and then deleted on the record. Objects: ADR 0003; the planning
   record in git history (deleted at handoff per ADR 0001).
2. **NORTH-STAR.md is never edited without operator consent on the
   concrete text** — operator ruling after the orchestrator overshot;
   recorded in the RECORD section (git history) and agent memory.
3. **Reviews run as fresh, visible herdr panes** of the other lineage,
   never inline `codex exec` — operator ruling; five reviews ran that
   way this day.
4. **Test-value policy (three operator decisions):** every governed
   repo's north star carries a Test values section (classes and ranks,
   established via the north-star interview); binding per-class time
   caps live in ADR 0001 citing that ranking; the test-cost audit is a
   standing opening step of full-tier TEST-STRATEGY. Production side
   **landed in bb-skills** (`skills-gyi` closed, commit `ca48c96`,
   pushed); consumption side is planned:
   `ab-testvalues-consumption-d01`.
5. **abacus-plan ships with abacus, not via bb-skills** — portability
   is `abacus init` (operator direction). Objects:
   `ab-init-plan-5ka` (planning trigger) blocked by
   `ab-virgin-bootstrap-jjq` (the operator's manual walkthrough on the
   new machine, checklist in the bead — the observation run that
   sizes init; MVP-first applied to the installer itself).
6. **Interim manual-merge authority was a bounded grant** — "while
   I'm gone and before the autonomous merge machinery is built," for
   that drain session. It produced PRs 18–25 and is **expired**. See
   hazards.
7. Jot-review ran (operator-invoked): 9 notes curated — 6 discarded, 2
   beads (`ab-bundling-plan-4mu`, `ab-braindump-phase-wow`), one
   grep-to-zero sweep-clause check added to the abacus-plan skill's
   DECOMPOSITION section.

Unresolved operator choices, parked: the north-star global symlink
(`~/.claude/skills/north-star` does not exist; one-line ln -s to
bb-skills when wanted); the bb-skills store migration (`skills-3vx`);
the operator's twice-gestured full rethesis of the north star.

## 5. Durable work state

**Landed (all pushed):** PRs 18–25 — capture_status (18), CI workflow
+ skip guards + BR_REAL shim (19; CI job names **test, clippy, fmt**
are the required-checks contract for `.1`), src/land.rs policy module
(20), default-branch discovery + AGENTS.md land exception (21), drain
loop (22), land wiring with the forbidden-flags teardown invariant
(23), origin/HEAD discovery fallback (24), parallel-suite flake fix
(25). Suite measured at authoring: **89 passed, 0 failed, 0 ignored**.
Epic children .2–.7 closed with red-first evidence; worker-filed bugs
`ab-o22` and `ab-origin-head-discovery-vyt` and `ab-xpe` closed. In
bb-skills: `skills-gyi` closed at `ca48c96`, in sync with its origin.

**In flight:** nothing in abacus. In bb-skills, the operator's `w2W`
Claude session settled after landing skills-gyi; its tree still
carries modified `README.md` and `skills/north-star/SKILL.md` plus
three untracked review files — **in-flight state of that repo's own
workstream, not abandoned work; do not clean it up from an abacus
session.**

**Uncommitted:** abacus tree clean at authoring (recompute — the
`.beads` live-dirty pattern applies whenever lanes run).

**Planned (open beads, `br list`):** `.1` queue config (operator seat;
repo transfer and remote update already done; remaining: branch
protection naming test/clippy/fmt, merge-queue ruleset at limit 1,
eligibility fixture into tests/fixtures/), `.8` live validation
(operator seat; the one decisive observation: first enqueued PR leaves
the queue merged), `ab-testvalues-consumption-d01`,
`ab-braindump-phase-wow` (same footprint — natural bundle),
`ab-bundling-plan-4mu`, `ab-init-plan-5ka` ← `ab-virgin-bootstrap-jjq`
(operator, new machine).

**Parked:** jot queues — **abacus 12 pending** (drain workers captured
throughout; authoring agent expected 2 and probed 12 — treat worker
capture volume as normal), bb-skills ≥1 (rename residue). Curation is
operator-invoked only.

## 6. Ownership and boundaries

- `w2W` (bb-skills) is the operator's session; its repo, tracker, and
  uncommitted files belong to that workstream. Abacus sessions write
  that tracker only with the rollback binary and only for cause.
- `.1` and `.8` are operator seats (`seat:operator` gates dispatch).
- The `abacus-review` workspace (`w2V`, two idle codex panes) is
  disposable — operator may close.
- PR merges are the operator's gate (see hazard 1). Planning-trigger
  beads (`*-plan-*`) are operator-invoked planning conversations, not
  dispatchable lanes.

## 7. Hazards, holds, and negative instructions

1. **Do not merge PRs.** The manual-landing authority of 2026-08-15
   expired with that session. Release: a new explicit operator grant,
   or the queue going live (at which point merging is the queue's and
   `--admin` is forbidden always — ADR 0003's invariant, tested in
   teardown).
2. **bb-skills tracker: never write with br 0.3.2** (schema 5, refused
   by design). Use `~/.local/bin/br-0.1.45` with `--db`. Do not
   migrate as a side effect; `skills-3vx` holds the sanctioned paths.
   Release: skills-3vx closed.
3. **`.1`'s required-check names must be exactly the CI job names
   test, clippy, fmt** or every enqueued PR times out (the
   check-name/ruleset coupling; first suspect if `.8` sticks).
4. **`land` against non-Rust repos is undefined** — the local
   admission leg is cargo-hardcoded. Release: the config surface from
   the `abacus init` epic.
5. **Machine-global hook fragility:** `~/.claude/settings.json`'s
   design-doc gate points at
   `~/dev-environment/bb-skills/hooks/design-doc-review-gate.py`. If
   that repo moves again, **every Edit/Write machine-wide blocks,
   including the hook's own repair** (observed once; escape was
   `sed` via Bash). The hook's internal role-card paths are still
   stale — jotted in bb-skills' queue.
6. Standing: reconcile-close retired (ADR 0002); `seat:operator`
   labels gate dispatch; commit `.beads` at natural points and pull
   with `--rebase --autostash`; review dispatches carry the no-bead
   instruction.

## 8. Incoming boot sequence

**Branch on machine identity first:**

```sh
command -v br || echo VIRGIN-MACHINE
```

**Virgin machine** (br absent): your mission is supporting the
operator's `ab-virgin-bootstrap-jjq` walkthrough. Read it without br:

```sh
grep '"id":"ab-virgin-bootstrap-jjq"' .beads/issues.jsonl | python3 -m json.tool
```

Its description is the full dependency-ordered checklist with known
potholes (br has no install recipe — copy the binary; repo-local git
config does not travel; remote URLs need the `git@` user). Capture
every friction point; that evidence unblocks `ab-init-plan-5ka`.

**Established machine:**

```sh
cat NORTH-STAR.md
br ready && br list --status in_progress
git -C ~/dev-environment/abacus status --short && git log --oneline -3
gh pr list --state open
```

**First consequential act:** none is urgent — the board is drained.
The highest-leverage next step is the operator's `.1` (queue config),
which unblocks `.8` and makes auto-merge real; support it when asked.

## 9. Verification ledger

| Claim | Probe | Observed (authoring, 2026-08-15 ~15:15 -04:00) | Incoming action |
|---|---|---|---|
| main equals origin/main | `git rev-parse HEAD origin/main` | both `d2233bc` | Recompute |
| Suite green | `cargo test` | 89 passed, 0 failed, 0 ignored | Recompute |
| No open PRs | `gh pr list --state open` | `[]` | Recompute |
| Open beads | `br list --status open --status in_progress` | exactly the 8 named in §5 | Recompute |
| Jot queue depth | `jot list` | 12 pending (abacus) | Operator-invoked review only |
| skills-gyi landed | bb-skills `git log -1`; `br-0.1.45 show skills-gyi` | `ca48c96` pushed; CLOSED | Trust as durable |
| bb-skills store schema | any `br` 0.3.2 command there | refused, "found 5" | Trust until skills-3vx |
| bb-skills agent | `herdr agent get w2W:p1` | idle post-landing | Recompute — volatile |
| Installed binary currency | PR 25 diff was tests-only | binary = PR-24 source, functionally current | Trust; reinstall after next engine PR |

**Corrections at authoring:** the outgoing agent's recalled jot count
(2) was wrong by 10 — worker lanes capture continuously during drains;
probe, never recall. No other defects found in this report at
authoring.

## 10. Closeout pointer

This report is the index; ADR 0003, the amended north star, AGENTS.md,
the bead descriptions (which deliberately carry their full context —
the planning record is deleted per ADR 0001), and the PR trail 18–25
are the cargo. On intake mismatch, stop only the affected action and
correct this file durably. The outgoing session commits this report,
pushes, and ends.

# Shift report — 2026-08-14 (orchestrator handoff, abacus)

Supersedes `SHIFT-REPORT-2026-08-13-CLAUDE.md`, deleted from the tree in
this report's commit; git history is the archive.

## 1. Identity and snapshot boundary

Repository `/home/ddc/dev-environment/abacus`, branch `main`, remote
`origin` attached and equal at authoring. Outgoing: the Claude
orchestrator (workspace `w1M`) that ran 2026-08-13 through 2026-08-14.
Incoming: the next orchestrator session. **Pre-report base:** `7ae9ccd`
(equal to `origin/main` at authoring, 2026-08-14 ~21:50 -04:00). Resolve
this report's own commit after intake:
`git log --oneline -1 -- SHIFT-REPORT-2026-08-14-CLAUDE.md`.

## 2. Read-first authority map

- **`NORTH-STAR.md`** — the thesis, amended 2026-08-14 by operator revise
  mode: planning is inside the product. The amendment log carries the
  prior blob and rationale.
- **`docs/adr/0001-planning-flow.md`** (accepted, amended same day) — the
  planning flow: tiers, six substages, lifecycle-classed planning state,
  artifact-conditional RECORD. **`docs/adr/0002-shared-work-state-store.md`**
  (accepted) — one shared `br` store for all lanes, wrapper carriage,
  close-last protocol. Both carry full review trails in their status
  lines.
- **`CONSTRAINTS.md`** — the four measured findings; still binding.
- **`AGENTS.md`** — the worker contract: lanes, protocol, the installed
  `br` wrapper, the review-dispatch no-bead rule, the `ab-*`/`abacus-*`
  namespace split.
- **`.claude/skills/abacus-plan/SKILL.md`** — the planning flow skill,
  implements ADR 0001 as amended; measured once against its success bar
  (record on the closed `ab-yfv` epic).
- **Work state:** `br` (now 0.3.2 — see §4) in `.beads/`; the corpus
  structure is declared in `docs-corpus.json` and enforced by
  `docs-doctor` (clean at authoring).

No authority inversion is active.

## 3. Objective and success condition

Unchanged from the north star: a backlog of N ready beads drains to
closed **overnight across two repositories with zero operator
interventions**, engine-resolved merge conflicts, clean PRs by morning.
This session built and live-validated the full single-repo loop —
select (label-filtered), claim in the shared store, dispatch, retry lost
prompts (two signatures), verify outcome from bead state, reap, worker
opens the PR, lane duration reported. **The overnight multi-bead run has
not yet been attempted; it is the next milestone.** Expressly out of
scope: merging to main (operator's gate), fixes inside `br` upstream.

## 4. Direction changes and settled decisions (apply, do not re-litigate)

Chronological, each with its durable object:

1. **Planning entered the thesis** — north-star amendment 2026-08-14
   (prior blob in its log). Supersedes non-goal 3 of the founding text.
2. **ADR 0001 accepted, then amended the same day:** quick-tier planning
   state lives on the epic bead; a full run keeps one root
   `PLANNING-<epic-id>.md` committed per gate and deleted at handoff;
   RECORD is artifact-conditional (substage and gate always run);
   quick-tier test method absorbed grep-first / negative-space / concrete
   cases. Object: the ADR's Amendments section.
3. **docs-doctor corpus adopted** (operator direction): `docs/planning/`
   retired; `docs/INDEX.md` is the corpus map. Object: `docs-corpus.json`
   plus the `ab-gi3` epic (closed).
4. **ADR 0002 accepted:** all lanes use the main checkout's `br` store;
   lane branches never touch `.beads`; the close is the worker's last
   act after push and PR; the orchestrator's reconcile-close practice is
   **retired** (it manufactured tracker conflicts). Objects: the ADR and
   the closed `ab-nl5` epic with live validation on its own delivery
   lane (PR 15).
5. **Wrapper carriage installed:** `~/.local/shims/br` symlinks
   `bin/br-shim`; `~/.zshrc` prepends the dir. Verified three-way.
   Object: closed `ab-nl5.3` notes and AGENTS.md.
6. **br upgraded 0.1.45 → 0.3.2** (upstream main commit `5154a379`,
   untagged — watch for the `v0.3.2` tag to formalize the pin). The
   store was rebuilt from committed JSONL; nine legacy `abacus-*` ids
   preserved. Full trail incl. toolchain requirements (nightly),
   rebuild recipe, behavior changes, and rollback rails: closed
   `ab-qb3` notes. This resolved `ab-72p` (labels now in
   `ready --json`) and unblocked the workaround removal (PR 16, merged).
7. **Bundling** (multiple same-footprint beads, one lane, one PR) is an
   operator-approved *manual* dispatch pattern with two successful runs;
   as an engine feature it awaits a planning run — held in the jot
   queue with its evidence.

Unresolved operator choices: none blocking. The jot queue holds four
notes for the next operator-invoked `/jot-review`.

## 5. Durable work state

**Landed** (all pushed; PRs 1–16 merged by the operator): the engine
binary features above; the planning skill; both ADRs; the corpus; the
shim. Suite at `7ae9ccd`: 42 tests, clippy and fmt clean, full run
~2.5s against the 30s budget.

**In flight:** `ab-mk9` — probe-retry composition fix (a transient
`br show` failure at probe time must not mask the never-engaged
re-prompt). Codex worker in herdr workspace labeled `ab-mk9`
(lane `w2H`), status `working` at authoring. **Two known complications**
for whoever lands it: (a) its branch was cut from pre-PR-16 main, so a
small `src/main.rs` conflict at its PR is expected — pre-merge with
main before or after review; (b) the `abacus run` process that
dispatched it waits in the outgoing session and may not survive session
end — the lane itself persists in herdr; if the waiter is gone, probe
the outcome manually (`br show ab-mk9` from the main checkout) and reap
with `herdr worktree remove` after verification.

**Uncommitted:** `.beads/issues.jsonl` is **live-dirty by design**
whenever a lane runs — workers write the shared store through the shim.
Not abandoned work. Commit tracker state at natural points; use
`git pull --rebase --autostash`.

**Planned (not started):** the overnight multi-bead run; the bundling
planning run; upstream `br` request for worktree-aware store discovery
(would retire the shim; no local br repo exists — operator routes).

**Parked:** four jot notes (bundling+evidence; `database busy` probe
class — largely addressed by `ab-mk9` once landed; a stale doc
reference on closed `ab-yfv.1`; the live-dirty-jsonl cadence wrinkle
from this evening). Curation is operator-invoked only.

## 6. Ownership and boundaries

- Lane `w2H` belongs to the `ab-mk9` codex worker until its outcome is
  verified. All other lanes are reaped.
- `~/dev-environment/skills` and `~/dev-environment/jot` belong to the
  knowledge workstream — read, do not edit without coordination. One
  forwarded note sits in the jot repo's own queue (sandbox capture
  loss), delivered at operator direction.
- PR merges and jot-review invocation are the operator's acts.

## 7. Hazards, holds, and negative instructions

- **Do not reconcile-close beads.** Retired practice; closes arrive via
  workers through the shared store. Release condition: none — this is
  the standing protocol under ADR 0002.
- **Until `ab-mk9` lands**, a lost startup prompt combined with a
  transient probe failure needs manual recovery: re-prompt the lane with
  the protocol text (see any recent lane prompt in `ab-mk9`'s
  description lineage), then probe and reap manually. Release: `ab-mk9`
  merged and the binary reinstalled (`cargo install --path .`).
- **Reinstall after merging engine PRs** — the installed binary is the
  engine; a merged fix is inert until `cargo install --path .` runs.
- **The br pin is an untagged commit** (`5154a379`). Do not `br upgrade`
  casually: 0.3.x refuses cross-schema migration, and the rebuild recipe
  (in `ab-qb3` notes) plus rollback rails
  (`~/.local/bin/br-0.1.45`, `.beads/beads.db.schema5.bak`) are the
  safety net. Release: upstream tags v0.3.2 or later and a fresh
  verify-on-copy pass approves it.
- **`seat:operator` labels gate dispatch.** Operator-run or
  orchestrator-seat beads must carry the label at creation or the engine
  will hand them to a codex lane (observed once, recovered).
- **Review dispatches must carry the no-bead instruction** (AGENTS.md
  rule) or reviewers mint tracker exhaust.

## 8. Incoming boot sequence

```sh
cat NORTH-STAR.md
br ready && br show ab-mk9
herdr agent get ab-mk9        # lane alive? status?
git -C ~/dev-environment/abacus status --short && git log --oneline -3
docs-doctor --repo ~/dev-environment/abacus --json
```

**First consequential act:** settle `ab-mk9` — if its worker finished,
verify from the main store and the branch, handle the expected
`src/main.rs` pre-merge, see its PR through, reap the lane, reinstall
the binary. Then the board is clean for the overnight-run milestone.

## 9. Verification ledger

| Claim | Probe | Observed (authoring, 2026-08-14 ~21:50 -04:00) | Incoming action |
|---|---|---|---|
| main equals origin/main | `git rev-parse HEAD origin/main` | both `7ae9ccd` | Recompute |
| One bead open, in flight | `br list` | `ab-mk9` in_progress only | Recompute |
| mk9 worker alive | `herdr agent get ab-mk9` | `working` | Recompute — volatile |
| Corpus clean | `docs-doctor --json` | `clean` | Recompute after any doc change |
| Suite green | `cargo test` | 42 passed across 7 suites | Recompute |
| br version and store | `br --version`; `br stats` | 0.3.2; 39 issues | Trust; recipe in ab-qb3 notes |
| Shim active for lanes | closed `ab-nl5.3` notes; PR 15/16 lanes wrote main store live | closes arrived mid-run | Trust as durable |
| Jot queue | `jot list` | 4 pending | Operator-invoked review only |

**Corrections:** none discovered in this report at authoring. The
founding report's rename-breaks-worktrees lesson remains true and lives
in git history with that report.

## 10. Closeout pointer

This report is the index; the ADRs, CONSTRAINTS.md, AGENTS.md, the
skill, and the br trail are the cargo. On intake mismatch, stop only the
affected action and correct this file durably. The outgoing session
commits this report, pushes, and ends — it holds nothing not written
here or in the objects this report names.

```doc-meta
role: contract
lifecycle: active
```

# ADR 0004: Foreign-repository onboarding — tracker migration, worker contract, and boundaries (market-brief-package)

- **Status:** **proposed** 2026-08-17 — pending Codex spec validation
  (operator direction of this date; both operators froze all codebase
  actions in both repositories until review completes and the operator
  rules). Trail: bloat review ran the same day (fresh Codex pane, one
  pass, seven cuts) and the operator **set its output aside wholesale**
  — the cuts were anchored to this repository's north star, while the
  document serves the target repository's mission, whose own north star
  is not yet established (the operator's declared first step after
  unfreeze). The anchor gap is jotted as methodology evidence. Spec
  validation proceeds on the unmodified document. One action predates
  the freeze and is complete: the backup/archive leg (D3), executed and
  verified before the freeze was declared.
- **Date:** 2026-08-17
- **Deciders:** operator (all rulings below), orchestrator session
  (record), with the resident market-brief-package Claude session as
  negotiating counterparty and adversarial reviewer — six rounds of
  measured cross-review; every measurement cited here is theirs or was
  independently reproduced.
- **Authority:** NORTH-STAR.md thesis (simultaneous multi-repo
  operation is a constraint, not a detail); ADR 0001 (planning flow;
  evidence-before-tooling); ADR 0002 (shared store, close-last
  protocol); the operator rulings of 2026-08-17 recorded per decision
  below.

## Context

The operator directed onboarding `~/dev-environment/market-brief-package`
(GitHub `DDC-Heartwood/market-intelligence`, Python/JS monorepo) as the
second abacus-governed repository. **The primary mission is production
use** (operator reframe 2026-08-17): the operator needs this tooling
working in that repository to get real work done now. It is also the
first onboarding of a repository abacus did not grow up in, and the
operator ruled it proceeds manually — so it doubles as the observation
run for the per-repo half of the `abacus init` epic
(`ab-init-plan-5ka`), with every friction point captured to the jot
queue rather than fixed ad hoc (MVP-first ruling); that evidence is a
byproduct, not the driver.

The target repo differs from abacus in every dimension that matters: a
bd-era Dolt tracker (1465 issues, 17 `bd remember` memories) instead of
br; a working branch (`llm-integration`) that is not the GitHub default;
an agent contract (AGENTS.md) that has drifted against standing operator
directives for five weeks; a second, larger agent contract (GEMINI.md);
no docs-doctor manifest; editable-installed Python packages that
interact non-obviously with worktree-based lanes; and a SABLE-era
gating stack that the operator has ruled retire-not-fix machine-wide.

Work state: epic `ab-mbp-onboard-hb0` (children .1–.6) carries the full
execution spec; this ADR records the decisions and their grounds.

## Decisions

**D1 — Manual onboarding as evidence purchase.** No onboarding tooling
is built for this instance. The manual sequence, its friction, and its
corrections feed `ab-init-plan-5ka` (nine jots filed to date, including
the per-repo config surface, path-conditional gates, out-of-tree
archives, and the import-resolution check any Python target needs).

**D2 — Tracker migration, scoped import.** The bd/dolt store migrates
to br with prefix `mb-` for new mints; imported beads keep their
original ids verbatim (mixed-prefix precedent: this repo's own store).
Import filter: status in {open, in_progress, deferred} ∪ (closed since
2026-07-17), measured at 351 beads — imported in full; deletions are
post-import hygiene per D9, never an import-time subtraction.
Grounds for the filter's shape: there is NO stored `blocked` status
(bd derives it over open; a `status=="blocked"` clause matches zero
rows while reading as coverage), and the 39 `deferred` beads are a
deliberate not-now queue that br preserves first-class — excluding
them would convert "later" into "never" as a side effect of wording.
Dangling dependency edges to archived beads (23 measured): drop the
edge, record the archived id in the importing bead's notes. Prose
citations outweigh edges (30 citations from 26 imported beads vs 2
dangling edges on the unexpanded filter) — hence D3's greppable layer.
Memories are NOT imported (no br equivalent exists): 6 live standing
directives transcribe to an interim standing-directives document whose
name and survival are an open operator review; 11 expired
session-state memories stay archive-only; the two prod-verification
near-duplicates dedupe to one; the memory that sunsets with bead
`j4yzw` keeps that linkage explicit.

**D3 — Archive out-of-tree, restore-verified, tool-independent layer
named. (Executed pre-freeze.)** All archive artifacts live at
`/home/ddc/dev-environment/beads-archives/market-brief-package/`,
never in any repo working tree — `git clean -fdx` deletes untracked
AND ignored files, which a fleet-worktree'd repo makes a live hazard.
Contents: the premigration tarball (restore-verified: extracted,
served, counts matched source), the Dolt-native backup (restore path
REQUIRES bd 1.0.5, a binary this machine is retiring — pinned in the
archive README), the uncompressed full export JSONL (1482 lines: 1465
issues + 17 memories; verified twice against independent exports,
identical id sets), an id→title index, and a protective copy of the
June 2026 tarball. The JSONL + TSV are the tool-independent layer that
keeps prose-cited bead ids resolvable after bd and dolt are gone.

**D4 — Worker contract: full AGENTS.md replacement.** Exhibit:
`PLANNING-mbp-agents-contract-draft.md` (v3, adversarially reviewed by
the resident session; their blocker was raised, self-corrected, and
the corrected diagnosis is what the exhibit encodes). Core content:
the lane protocol with durable BLOCKED (`br comments add` before
stopping — a blocked lane never reaches `br close`, and stdout gets
lost); path-based test-gate selection (a fixed default gate yields
greens unrelated to the change under test — the meaningless-green
class this repo was burned by twice); the import-leak discipline (run
gates from the worktree root — editable finders append after
`PathFinder`, so cwd wins when it contains the package; the leak fires
only on `cd`-before-invoke, which CI's own workflow file would teach a
parity-seeking lane; PYTHONPATH prepend as verified fallback; never
`pip install -e` from a lane worktree, which repoints the shared
environment at a reapable worktree); red-first evidence as instruction
checked at PR review, with NO hook-enforcement claims (tdd-gate.sh is
bd-only, SABLE-era, retire-not-fix); the Docker/Supabase integration
leg as a stated exemption; both `bd dolt push` instances removed (one
sat OUTSIDE the bd-managed fence), with the chuck-only directive's
authority chain preserved by name.

**D5 — Default branch flips to `llm-integration`.** The engine
discovers the default branch from origin/HEAD and workers PR against
it; no per-repo override surface exists (recorded init evidence, not
built). The flip also closes the resident repo's known live defect
(`dgu78`: stale default, workflow_dispatch 404s). CI already triggers
on llm-integration.

**D6 — No landing path is assumed.** `abacus land` stays OFF for this
repository (cargo-hardcoded admission; operator does not intend
autonomous merging there). Lanes end at PRs. The repo's sole
sanctioned merge path (`sable-merge-gate`) is SABLE-era; its successor
ruling is open in the resident session. PRs waiting at the human merge
boundary is the permitted MVP shape, so drains (including the smoke
lane, whose PR deliberately stays unmerged) do NOT wait on the
successor ruling — that ruling owns only eventual landing and the
timing of D9's hygiene pass.

**D7 — Skill distribution by symlink.** `abacus-plan` reaches all
repos on this machine via a global skills symlink; `abacus init`'s
copy story narrows to cross-machine portability.

**D8 — docs-doctor conformance is mandated for the target repo.**
Sized by the resident session at ~2h mechanical (25 managed docs, no
manifest, no INDEX, zero doc-meta blocks) plus three content decisions
reserved to their operator. Gated on that session's commit-authority
wave. The exhibit's own doc-meta block is not claimed conformant until
the manifest exists and doctor runs green.

**D9 — Backlog triage before import.** The ready queue is not
drainable as-is: pivot-stale P1s (parked ADR 0008 lineage), agenda and
meta beads, and 117 SABLE-mentioning beads (33 of 101 active). The
operator ruled exclusively-SABLE beads are deleted from the live
backlog. **Deletion is decoupled from the import** (operator reframe
2026-08-17): beads import per the filter, and exclusively-SABLE ones
are deleted as post-import backlog hygiene once the successor ruling
lands and the resident session's classification extends — meanwhile
per-run drain-set selection keeps stale beads out of lanes, so the
migration does not wait on the ruling. Deleted beads remain greppable
in the archive. Three beads (`zh4gt`, `i3xzk`, `9ml7z`) describe
gating failure modes in general and transcribe into any successor
gate's spec before leaving the live pool, whatever their dispositions.

**D10 — Two-session negotiation with seat-scoped authority.** The
orchestrator and the resident session negotiate and cross-review;
neither acts on the other's relayed operator authority (twice
exercised: GEMINI.md, the global-config edit); each operator decision
is signed off in the session that owns the seat. This protocol caught,
before execution: two spec bugs in the migration filter, the archive
placement hazard, the bd-1.0.5 restore dependency, the false
symlink-equivalence premise on GEMINI.md, an overstated blocker
(retracted by its own author on re-measurement), and the import-leak
class itself.

## State at freeze (2026-08-17, updated same day)

Both trees clean of onboarding changes. market-brief: branch
`llm-integration`; the resident operator granted commit authority and
the formerly-frozen `xiv8r` diff **landed at `2c32fd2`** (gates green;
an explicit WIP checkpoint — resolver + seven tests complete,
`set_relation_span` not yet wired into the paragraph schema or
renderer, final adversarial pass outstanding), leaving that working
tree fully clean; dolt server cleanly stopped (pid/port absent);
nothing else applied. The resident session deliberately did NOT write
the tracker, so the store still matches the verified backup and export
exactly — but the exported `xiv8r` bead describes a frozen uncommitted
diff, stale against git; the migration bead requires appending the
`2c32fd2` correction to the migrated bead immediately after import.
abacus: epic + tracker records committed and pushed. Archive intact
(six entries incl. README). Closed: `ab-mbp-onboard-hb0.1` (backup).
Global CLAUDE.md bd→br draft exists (session scratchpad), applies at
migration time, unapplied.

## Open questions (for review and operator ruling)

1. `sable-merge-gate` successor — owns D6's release and the timing of
   D9's post-import hygiene pass (no longer a migration gate).
   Resident-session seat.
2. GEMINI.md — deletion premise refuted; keep-and-trim recommended;
   operator re-decision pending.
3. Standing-directives document — final name, location, survival.
4. conftest.py import-resolution guard — the durable fix for the D4
   leak class is repo-side, a few lines, and protects humans and lanes
   alike; reviewers should say whether it belongs in this ADR's scope
   or the resident backlog. (Posed as a question, not a decision.)
5. Global CLAUDE.md bd→br edit — drafted; timing pinned to migration.
6. Target-repo north star — the operator's declared first step after
   unfreeze (establish mode, resident session). Not required by the
   execution path (the engine never reads it); required before the
   first full-tier planning run there and before design-doc reviews in
   that repository can anchor to anything — this ADR's own bloat pass
   demonstrated the anchor gap. Mechanical prerequisite: the global
   `~/.claude/skills/north-star` symlink does not exist yet (one-line
   `ln -s` to bb-skills).

## Consequences

Accepted: onboarding cost is paid manually once, as evidence. The
import excludes ~76% of historical beads from the live pool — the
archive's greppable layer is the compensating control. Structural,
discovered by the first lane: bd's store was gitignored while br's is
committed by design, so migration converts the entire bead corpus into
scanned repository content — any onboarded repo with a repo-wide
content guard hits this on day one (market-brief: 67 violations,
every markdown lane blocked). Onboarding a guarded repo must update
the guard's exclusions as part of the migration, not leave it to be
discovered by the first lane; recorded as init-epic evidence. The worker
contract grows repo-specific length (gate table, step-zero) — accepted
because every line traces to a measured failure mode. Multi-repo
operation becomes real: one machine, one herdr server, two governed
stores — the north-star constraint gets its first genuine test.

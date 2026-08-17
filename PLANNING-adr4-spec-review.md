# ADR 0004 spec-validation findings

Verdict: four findings. The document is not implementable as written until the
blocker and contradictions below are reconciled. This was a refinement-only
review; the set-aside bloat review was not used and scope was not reconsidered.

## Findings

### 1. Blocker — the `location-briefing` import oracle names no importable module

`PLANNING-mbp-agents-contract-draft.md`, “Step zero — import-leak discipline,”
lines 64–70 requires an oracle for every touched top-level package and gives:

> `python -c "import location_briefing, ..."`

The target package does not expose that module. Its packaging declaration says
`name = "location-briefing"` at
`market-brief-package/location-briefing/pyproject.toml:6`, but package discovery
is `include = ["src*"]` at lines 106–110. Running the exhibit's exact import
from the target repository root with bytecode writes disabled returns
`ModuleNotFoundError: No module named 'location_briefing'`.

Concrete failure: every lane touching `location-briefing/backend/` or
`location-briefing/src/` fails step zero before pytest, then reaches the
mandatory `BLOCKED` outcome at exhibit lines 71–79 even when its checkout is
correct. The oracle must name a module the target tree actually provides; the
current command cannot verify the accepted import-leak guarantee.

### 2. High — D6 still gates the first drain on a successor that the operator removed from the critical path

`docs/adr/0004-foreign-repo-onboarding.md`, D6, lines 129–134 ends with:

> “Successor-before-first-drain: a drain whose PRs nobody may land is pointless.”

The later operator ruling says the opposite. `br show ab-mbp-onboard-hb0`,
Comments, 2026-08-17 15:04 UTC records that decoupling SABLE deletion “removes
the sable-merge-gate successor ruling from the critical path entirely” and
lists the smoke lane on that critical path. `br show ab-mbp-onboard-hb0.6`,
Acceptance, expressly requires the smoke-lane PR to remain unmerged because no
successor exists yet.

Concrete failure: D6 blocks `ab-mbp-onboard-hb0.6` until the successor ruling,
while the current execution record runs that same first drain before the ruling
and intentionally leaves its PR open. The stale final sentence of D6 must be
reconciled with the later ruling; no new behavior is needed.

### 3. High — D2 subtracts triage deletions during import although D9 moved them after import

`docs/adr/0004-foreign-repo-onboarding.md`, D2, lines 66–70 defines the import
set as:

> “measured at 351 beads — minus triage deletions (below).”

The same ADR's D9, lines 147–156 says instead that “beads import per the filter”
and exclusively-SABLE beads are deleted only as “post-import backlog hygiene,”
so “the migration does not wait on the ruling.” The trail confirms the later
rule: `br show ab-mbp-onboard-hb0.2`, Notes says the SABLE-exclusion clause waits
for the post-successor classification and, “Do not use them as the
exclusive-vs-incidental filter”; the epic's 15:04 UTC comment likewise says the
deletion is not an import gate.

Concrete failure: following D2 waits for incomplete triage and imports fewer
than the 351-bead filter set; following D9 imports that set and defers deletion.
Those paths produce different live stores and different critical paths. D2's
pre-reframe subtraction must be removed or explicitly superseded by D9.

### 4. Medium — step zero and the frontend gate require mutually exclusive working directories

`PLANNING-mbp-agents-contract-draft.md`, “Step zero,” lines 57–63 says:

> “Run every gate from your worktree root. Never `cd` into a subdirectory
> first.”

It then names the backend gate as “the one legitimate exception.” The gate
table at line 97 nevertheless requires `npm run lint` and `npm test` “from
`location-briefing/frontend/`.”

Concrete failure: a frontend lane cannot obey both instructions. In a
multi-tree diff, following the table by changing directory also leaves the
worker in exactly the subdirectory state step zero forbids before a Python
gate. The frontend invocation must be expressed consistently with the existing
root rule, or the existing exception statement must acknowledge it.

## Not checked

- I did not redo scope/bloat review or apply any of the set-aside cuts.
- I did not inspect child `.1` or the out-of-tree archive, so D3's restoration,
  inventory, and count claims were not validated.
- I did not execute the migration, full test gates, docs-doctor, GitHub default
  branch change, or global symlink operation, and I did not independently
  recompute the tracker statistics.
- Target-repository inspection was limited to the package/import layout,
  frontend command location, and the exact failing import oracle; other gate
  commands and runtime behavior were not audited.
- I did not resolve or evaluate the operator-owned open questions in ADR lines
  191–211.

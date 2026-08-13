# Shift report — 2026-08-13 (founding handoff, new abacus repository)

## 1. Identity and snapshot boundary

Repository: `/home/ddc/dev-environment/abacus`, freshly `git init`-ed on
branch `main`, **no remote configured** — creating and attaching one is
the operator's act. Outgoing: the Claude orchestrator (`w1:p1`) that ran
the 2026-08-12 session in the repository now renamed `abacus-v1`.
Incoming: the first agent of this build.

This repository has **no pre-report base**: its first commit is this
report plus `NORTH-STAR.md`. Resolve this file's own commit after intake
with `git log --oneline -1 -- SHIFT-REPORT-2026-08-13-CLAUDE.md`.

Observation time for every probe below: 2026-08-13, authoring pass.

## 2. Read-first authority map

- **The thesis:** `NORTH-STAR.md` in this repository. It is the standard
  every proposal is judged against. It was established by operator
  interview, not written by an agent, and it is amended only through the
  north-star skill's revise mode.
- **The parts bin:** `/home/ddc/dev-environment/abacus-v1` at `8124dc3`
  (`origin/main` equal at authoring). It holds 46k lines of working,
  tested Rust — pinned-provider gates, a `br` adapter and record
  transport, a Herdr adapter, a checked-append core. Import from it when
  a working feature needs a part. **Never wholesale, never on the
  argument that it exists.**
- **Review roles:** `~/dev-environment/skills/agents/bloat-reviewer.md`
  and `spec-validator.md`, at skills-repo `5a23454`. Provider-neutral
  role cards; point any agent at the file.
- **The gate:** `~/dev-environment/skills/hooks/design-doc-review-gate.py`
  at skills-repo `1e2ddf3`, wired into both `~/.claude/settings.json` and
  `~/.codex/hooks.json`. Creating an ADR/PRD/PROPOSAL markdown injects
  the review discipline automatically.

There is **no authority inversion** in this repository. It is empty, so
nothing here is knowingly stale. That is the point of starting it.

## 3. Objective and success condition

Build the execution engine described in `NORTH-STAR.md`. The immediate
deliverable is not a design: it is **a program that runs**.

The first commit after this report should be the smallest thing that
performs the loop the outgoing session performed by hand roughly a dozen
times: read `br ready`, create a worktree, spawn a pinned Codex pane,
send a dispatch prompt, wait for it to settle. Shelling out to `br` and
`herdr` is expected and sufficient. Records, acceptance, and evidence
chains are explicitly NOT in the first commit — `br` already holds
status and the agents already report.

Success for the product is `NORTH-STAR.md`'s success condition. Success
for the first week is that something ran and produced a failure worth
fixing.

**Expressly out of scope:** porting `abacus-v1` forward; re-deriving its
ADRs; writing a CONTEXT.md; anything the thesis names a non-goal.

## 4. Direction changes and settled decisions

All from the 2026-08-12 session. Apply these; do not re-litigate.

1. **The `yx3` deletion arc is stopped.** Prior direction: eight
   C3-class children removing the transitional stack, gating an
   interface freeze. Replacement: stop after C0 and C1, restart in this
   repository. Evidence: at the stop point `abacus-v1` held 46,094 lines
   of Rust, roughly 42% (19,457) already inventoried for deletion by the
   arc's own final child, while **zero** lines existed on the path to
   the success condition — `abacus-cli` contained only a README.
   Durable object: `abacus-yx3` comments in `abacus-v1`.
2. **The interface freeze is abandoned as a gate**, not deferred.
   Freezing seams before the product runs is backwards; the first
   overnight run is what teaches which seams matter.
3. **MVP first, fix as we use** (operator ruling, binding). Ship the
   braindead-simple version, dogfood it, handle real failures when
   observed evidence exists. Do not pre-build contingencies for failure
   modes that have not happened.
4. **Worker lanes default to Codex**; Claude is the orchestrator seat and
   the cross-lineage review leg. Lineage independence is evidenced, not
   assumed: on an identical scope-review task with an identical role
   card, a Claude reviewer concluded "no whole child is cuttable" while
   a Codex reviewer independently proposed cutting the two children the
   operator and orchestrator had separately concluded were unnecessary.
5. **The SABLE hooks were removed from this machine** (2026-08-13). 53
   hook entries across both agent configs. Consequences that matter
   here: there is **no tree-claim coordination** and **no pre-push test
   gate** — running gates before pushing is now discipline, not
   enforcement. Kept: the design-doc gate, the `.env` read guard
   (Codex), Herdr agent-state, screenshot tooling. Recovery: the scripts
   are tracked in the `SABLE` repo and the removed config is backed up
   at `~/.claude/settings.json.bak-presableremoval` and
   `~/.codex/hooks.json.bak-presableremoval`.

**Unresolved and carried as debt** (none blocks the first commit):
credential-carriage timing; whether planning eventually enters the
thesis (the operator intends it, deliberately excluded it from this
thesis, and the non-goal is revisitable through revise mode).

## 5. Durable work state

**Landed** — each object with what it proves:

| Object | Proves |
|---|---|
| `abacus-v1` `8124dc3` | the parts bin, `origin/main` equal at authoring |
| `abacus-v1` `039869e` | C0: checked-append core + production record bridge |
| `abacus-v1` `dc5f365` | C1: `ports.rs` retired, launch identity narrowed |
| branch `lane/yx3-c3-native-claim` `5af7a57` | C3 native claim, **UNREVIEWED**, pushed, never merged |
| skills `5a23454` | both review role cards |
| skills `1e2ddf3` | the design-doc gate, verified live on both agent kinds |

**In flight:** nothing. No lane is running for this repository.

**Uncommitted:** nothing in this repository.

**Planned** (not started, do not describe as existing): the `abacus run`
skeleton; a short constraints file carrying the four measured findings
in section 7; a one-page `AGENTS.md`.

**Parked:** the five open `yx3` children in `abacus-v1`
(`abacus-atb`, `-nda`, `-dsy`, `-rii`, `-ai5`). They remain `open` and
P1 in that tracker **by neglect, not by intent** — the stop is recorded
on `abacus-yx3` but the children were not individually closed. Do not
read `br ready` in `abacus-v1` as a work queue.

## 6. Ownership and boundaries

- **This repository** has no lanes and no history. The incoming agent
  owns everything in it.
- **`abacus-v1`** is read-mostly. Its tracker still accepts records, but
  no new build work happens there.
- **Off limits:** `~/dev-environment/skills` and `~/dev-environment/jot`
  belong to the knowledge workstream (`w1J`, `w1H` panes). The review
  cards and the gate live in `skills` — read them, do not edit them
  without coordinating.
- **Not yet done, and the operator's act:** creating the GitHub
  repository and attaching a remote. Until then this repository exists
  only on this machine.

## 7. Hazards, holds, and negative instructions

- **Do not import from `abacus-v1` wholesale.** Code enters when a
  working feature needs it, and it enters as-is. Re-reviewing imported
  code on arrival is how the accretion returns.
- **Do not carry `CONTEXT.md`, the ADRs, or the beads across.** They are
  the mechanism by which prior scope became authority. Carry instead
  only these four **measured** findings:
  1. `br`, not `bd` — at 11 concurrent claimants `br` served 879 reads
     with zero timeouts (p50 51ms); `bd` recorded 15/15 read timeouts at
     similar width.
  2. Provider identity must bind to **every execution**, not startup —
     a cached verdict, a held inode, and an ambient staging root each
     defeated a naive gate, all three reproduced live.
  3. The worker launch environment must carry **bead and attempt**, or a
     context-lost worker cannot enumerate its own records.
  4. **Crash recovery is first-class on this host** — the operator's
     machine crashes; "requires a crash at the wrong moment" is not a
     mitigating argument here.
- **Anyone importing the C3 claim seam owes five guarantees** the audit
  found narrowed with no replacement: forged-content-hash refusal,
  substituted-scope-map refusal, bead-never-offered refusal, revision
  bracketing (narrowed to close only), and an exhaustive idempotency
  matrix (narrowed to close only). Detail on `abacus-to2` in `abacus-v1`.
- **No tree-claim coordination exists.** Concurrent lanes on one
  checkout have no mutual exclusion. Use a worktree per lane.
- **Round three on one artifact is a scope signal, not a fix signal.**
  Three consecutive landings in the last session carried a coverage
  defect that passed its own green gates. The listed-case delta and a
  `cargo test -- --list` roster diff are the two cheap tripwires; a
  third class — assertions thinned inside retained test names — is
  invisible to both and needs a reviewer.

## 8. Incoming boot sequence

```sh
cat NORTH-STAR.md                                   # the standard
git -C ../abacus-v1 log --oneline -3                # the parts bin
ls ../abacus-v1/abacus-runtime/src ../abacus-v1/abacus-work/src
cat ~/dev-environment/skills/agents/bloat-reviewer.md
git log --oneline -1 -- SHIFT-REPORT-2026-08-13-CLAUDE.md
```

**First consequential act:** write the `abacus run` skeleton described in
section 3 and make it execute against a real backlog. If the first
session instead produces an ADR, a CONTEXT.md, or an invariant list, the
gate will say so — and that is the signal to stop and reconsider, not a
formality to acknowledge.

## 9. Verification ledger

| Claim | Probe | Observed (authoring) | Incoming action |
|---|---|---|---|
| This repo is empty but for the thesis | `ls -a` | `NORTH-STAR.md` only, plus this report | Recompute |
| Parts bin tip | `git -C ../abacus-v1 rev-parse HEAD` | `8124dc3`, `origin/main` equal | Recompute |
| C3 preserved and pushed | `git -C ../abacus-v1 ls-remote --heads origin` | `5af7a57` present on remote (see correction 2) | Trust as durable |
| Cards and gate landed | skills `git log` | `5a23454`, `1e2ddf3` | Trust; reread cards |
| Gate fires on both agent kinds | live probe, fresh Codex session | quoted the injected guidance verbatim | Trust; retest if configs change |
| No lanes running here | `herdr agent list` | no pane in this repository | Recompute — volatile |
| 5 yx3 children still open in v1 | `br list --status=open` in `abacus-v1` | 5 | Do not treat as a queue |
| No remote configured | `git remote -v` | empty | Operator must attach one |

**Correction 2, found after this report first landed.** The ledger
originally claimed the C3 branch was "pushed, push accepted, trust as
durable", citing `git rev-parse` as the probe. Both parts were wrong.
`rev-parse` resolves a **local** ref and proves nothing about a remote,
and the "acceptance" was GitHub's `pull/new/` hint in the push output
rather than a verified remote ref. A subsequent `ls-remote` showed only
`main`: the branch was **not** on the remote. It was re-pushed and
verified — `5af7a57` now resolves under `refs/heads/` on origin. The
lesson is the one this skill states directly: match the probe's object
to the claim, because containment in `HEAD` does not prove containment
in a remote branch, and a hint printed by a server is not a probe.

**Corrections from the authoring pass.** Renaming the old directory
broke every linked worktree's gitdir pointer; `git worktree repair` in
`abacus-v1` fixed them, and the C3 work would have been unreachable
without it. The lesson generalizes: a rename silently invalidates linked
worktrees, and the failure is a `fatal: not a git repository` that looks
like corruption rather than a moved path.

## 10. Closeout pointer

This report is an index, not a replacement for `NORTH-STAR.md`, the
parts bin, or the operator's instructions. On intake mismatch, stop only
the affected action, preserve the discrepancy, and correct this file
durably rather than explaining it in chat.

The outgoing session commits this report and ends. It holds nothing that
is not written down here.

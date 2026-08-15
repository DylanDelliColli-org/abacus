```doc-meta
role: planning-inflight
lifecycle: inflight
```

# PLANNING — ab-automerge-2b2 (PR validation and auto-merge machinery)

**Tier: FULL** — confirmed by operator 2026-08-14.

**Rationale:** the ask introduces new contracts (merge policy, validation
gate, risk-tier classification), carries real unknowns (moving-base
dynamics during a multi-bead run, validation semantics for an unattended
merge), amends the thesis (NORTH-STAR non-goal 3 and the success
condition currently end autonomy at the PR), and has a blast radius
spanning the engine, the worker protocol, at least one ADR, and the
north star. Every Quick criterion fails.

**Operator direction seeding the ask (2026-08-14):** without engine-side
validation and merging, the base moves constantly during an overnight
multi-bead run. Beyond PRs-waiting-in-the-morning, the operator wants an
autonomous mode for lower-risk or lower-complexity projects where
auto-merge throughput matters more than morning review.

Substages append below: FRAMING, RESEARCH, ARCHITECTURE, TEST-STRATEGY,
RECORD, DECOMPOSITION. This file is deleted at handoff; git history is
the archive.

---

## FRAMING

Produced live with the operator, 2026-08-14. Seven load-bearing
decisions were put to the operator explicitly (four in the first pass;
three more after the operator widened scope to include CI/CD mid-gate).
All answers are recorded under "Operator decisions" below.

### User stories

- **S1 — opt-in mode.** As the operator, I mark a repository as
  auto-merge eligible, so an overnight run on it merges validated PRs
  without me. A repository without the mark behaves exactly as today.
- **S2 — validate against the landing base.** Every auto-merged PR is
  validated against the main it will actually land on: the engine brings
  the branch up to date with current main and requires validation green
  *after* that update. Validation has two legs — the local leg (full
  suite, clippy, fmt on the updated branch) and the remote leg (GitHub
  CI checks green on the updated head, S7). A PR that was green when
  the worker pushed, but is stale against moved main, can never land on
  its old evidence.
- **S3 — serialized merges.** The engine merges one PR at a time; each
  subsequent PR revalidates against the post-merge base. The moving base
  becomes the mechanism rather than the hazard.
- **S4 — park on failure.** A PR that fails validation is parked: left
  open with the failure evidence attached, never merged, never
  discarded. The run continues with other work. Parking is the safety
  net for every failure class, including a conflict resolution whose
  result does not validate green (S6).
- **S5 — default unchanged.** Without the opt-in, autonomy still ends at
  the PR and morning review is untouched.
- **S6 — engine-resolved conflicts, in the wedge.** When updating a PR
  onto moved main hits a merge conflict, the engine resolves the
  conflict itself (approach to be locked in ARCHITECTURE; candidate
  shapes include mechanical strategies and dispatching a resolution
  agent). The resolved result must then pass S2 validation before merge;
  a resolution that fails validation parks per S4.
- **S7 — CI is a co-validator.** The merge gate requires GitHub CI
  checks green on the updated head in addition to the local leg. The
  remote leg is canonical evidence that survives an operator-host crash
  mid-run; the local leg fails fast in seconds.
- **S8 — standard CI is a deliverable.** A standard workflow (test,
  clippy, fmt on PR and on main) ships in this epic, installed first on
  abacus itself. Auto-merge eligibility requires CI present on the
  repository.
- **S9 — repo-agnostic by design.** The merge queue works against any
  repository the operator runs it on; abacus is merely the first.
  Nothing in the machinery is special-cased to this repository.

### Non-goals

- Deploy-side machinery. Merge to main triggers whatever pipeline the
  repository already has; CD standardization is a future ask.
- Third-party distribution. Generality means repo-agnostic design, not
  install flows or external users; north-star non-goal 2 stands for
  this epic.
- Rollback automation for merged work.
- Per-bead risk scoring or complexity inference.
- Changes to the codex review seat or any operator-invoked review flow.

### Epic success metric

An overnight run on an auto-merge-enabled repository drains a backlog of
at least 3 beads to merged-to-main with zero operator interventions and
the main suite green in the morning.

### Narrowest valuable wedge

After a bead closes and its PR opens, the engine — in auto-merge mode
only — updates the branch onto current main, resolving a conflict itself
if one arises (S6), runs the local validation leg, waits for CI green on
the updated head (S7), merges on both-green, and parks on red.
Serialized, one PR at a time, gated by a per-repository opt-in flag on a
repository that has CI (S8).

### Prerequisites

No existing bead is a prerequisite. One in-run prerequisite: the
NORTH-STAR revise-mode amendment lands at RECORD, citing the ADR, before
any implementation child is authored at DECOMPOSITION. The amendment
covers non-goal 3 (merging to main — carved out for auto-merge mode), a
success-condition variant, and the merge-queue aspect of non-goal 4
(the queue is a general, repo-agnostic capability). The amendment is an
operator act.

### Operator decisions (2026-08-14)

1. **North-star amendment timing: at RECORD.** The amendment cites the
   locked decision record instead of licensing a design that does not
   exist yet. Direction is decided now; the act happens at RECORD.
2. **Validation is mechanical only.** Branch updated onto current main,
   then full suite, clippy, and fmt green. No agent review leg in
   auto-merge mode; a review leg remains available as a later risk-tier
   knob if observed need arises.
3. **Risk tier is a per-repository flag.** The operator declares a
   repository auto-merge eligible in engine configuration or invocation.
   Finer granularity (per-bead labels) only after observed need,
   per the MVP-first ruling.
4. **Conflict resolution is in the wedge** (operator override of the
   planner's park-first recommendation). The success condition already
   promises engine-resolved conflicts; the operator wants the wedge to
   honor it directly rather than parking conflicts for morning.
5. **CI joins the validation gate** (operator scope widening,
   mid-FRAMING). Both legs required: local suite after the update, and
   GitHub checks green on the updated head. Rationale: CI is becoming
   standard across the operator's repositories anyway, and remote
   evidence survives a host crash.
6. **Standard CI ships in this epic**, starting with abacus.
   Auto-merge eligibility requires CI present.
7. **CD stays out.** Merge triggers existing pipelines only; the engine
   builds nothing deploy-side.
8. **General capability, not rethesis** (operator scope widening,
   mid-FRAMING: "Abacus has reached the ceiling of its current North
   Star"). The merge queue is repo-agnostic by design; the beneficiary
   remains the operator. The RECORD amendment covers the merge-queue
   aspect of non-goal 4 within the current north star. A full rethesis,
   if pursued, is its own /north-star revise session later.
9. **No third-party distribution in this epic.** Non-goal 2 stands
   here regardless of any later rethesis.

---

## RESEARCH

Produced by a sherlock-type subagent, 2026-08-15, against HEAD `861ecc4`. Every finding is anchored to a path, symbol, or probed command output. All module fingerprints and bundle groups below are **PROVISIONAL** — ARCHITECTURE may invalidate the design they assume, and DECOMPOSITION re-derives final footprints.

**Headline:** three findings change the shape of the frame. (1) There is no multi-bead engine loop — `cmd_run` dispatches exactly one bead and exits, so the epic's success metric presumes machinery that does not exist. (2) `gh pr checks` exits 1 for both "checks failed" and "no checks configured", and `capture()` discards exit codes entirely, so the S7 gate cannot be expressed with the engine's current process seam. (3) 17 of 45 tests hard-require a `br` binary that has no installable recipe, which blocks the S8 workflow from running the same suite the local leg runs — directly weakening S7's premise that CI is *canonical* evidence.

### Prior art

**GitHub native merge queue.** Provides server-side serialization, speculative batching, and automatic dequeue on failure. Requires a protected branch with required status checks plus a merge-queue ruleset; this repo has neither (`gh api repos/DylanDelliColli/abacus/branches/main/protection` → `404 Branch not protected`; `gh api repos/.../rulesets` → `[]`). It is a genuine **alternative to this entire epic**, not a component of it, and ARCHITECTURE should explicitly adopt or reject it. Three reasons it does not fit the frame as written: it validates a *speculative merge commit* GitHub constructs, so S2's local leg has no artifact to run against; it owns ordering, which displaces S3's engine-owned serialization; and it has no hook for S6 — a conflicting PR is dequeued, never resolved. Confidence high on the mechanics, high on the repo-state facts.

**git rerere.** Unset at both scopes (`git config --get rerere.enabled` and `--global` both exit 1 with no output). Replays previously-recorded conflict resolutions. Genuinely well-matched to a serialized queue draining N PRs off one moving base, because the same hunk conflicts repeatedly across the queue — but it is an accelerator, not a resolver: it can only replay a resolution some other mechanism produced first. Confidence high.

**Merge strategies.** git 2.43.0; `ort` is the default. `-X ours` / `-X theirs` resolve conflicting hunks by side preference. Dangerous here in a specific, non-obvious way: they produce a *green tree* by silently discarding one side's semantic change, and S2's validation cannot catch it when the discarded change's test was discarded with it. Direction matters — on a base-into-branch update, `-X ours` discards **main's** hunk, throwing away already-merged work. Confidence high.

**In-repo prior art for mechanical resolution.** `merge_jsonl` (`src/main.rs:74`) is a semantic three-way merge driver for one file class, registered locally as `merge.beads-jsonl.driver = abacus merge-jsonl %A %O %B`. Its failure posture is the precedent worth carrying: it exits non-zero on any unparseable line and lets git leave a normal conflict (`AGENTS.md:49-51`). That is S4's park, already implemented once in this codebase for a narrower domain. Confidence high.

**Agent-dispatched resolution.** No in-repo prior art. The machinery exists though: herdr's two-step `worktree create` then `agent start --pane` (both confirmed independently available; `worktree create` takes no `--kind`), and `herdr worktree open --branch NAME` can attach a workspace to an existing branch.

### Engine findings

**`cmd_run` phase map** (`src/main.rs:177-306`), line-anchored:

| Phase | Lines | Symbols |
|---|---|---|
| resolve repo | 178-181 | `Path::canonicalize` |
| select | 183-188 | `br ready --json` → `parse_ready` → `select_bead` (`lib.rs:39`, `:45`) |
| claim | 189 | `br update <id> --claim` |
| lane open | 195-221 | `herdr worktree create` → `parse_worktree_created` (`lib.rs:147`) |
| agent start | 223-230 | `herdr agent start --kind codex --pane` |
| dispatch + settle | 232-245 | `dispatch_prompt` (`lib.rs:180`), `herdr agent prompt --wait`, `is_agent_prompt_stalled` (`lib.rs:105`) |
| probe | 247 | `probe_bead_outcome` (`:166`) wrapping `retry_probe_once` (`:152`) |
| classify | via probe | `parse_bead_outcome` → `classify_bead_status` (`lib.rs:86`) |
| retry | 251-258 | `retry_never_engaged_once` (`:134`) |
| reap | 259-284 | `should_reap_lane` (`lib.rs:98`), `is_dirty_worktree_remove_error` (`lib.rs:113`) |
| outcome | 286-299 | `BeadOutcome` match |

**There is no loop.** `cmd_run` selects exactly one bead (`select_bead` returns `Option<&ReadyBead>`, `main.rs:185`) and returns. `usage()` (`:51`) offers only `run [repo-path]` and `merge-jsonl`. The epic success metric — "drains a backlog of at least 3 beads" — therefore rests on machinery not yet built; today an overnight run means an operator or shell loop re-invoking `abacus run`. ARCHITECTURE must decide whether the multi-bead loop is in this epic's scope or a prerequisite. Confidence high. Verify: `grep -n "select_bead\|fn cmd_run\|loop" src/main.rs`.

**The engine does not know the PR exists.** `gh` appears in `src/` only as prompt text (`lib.rs:190`) and as a test assertion (`lib.rs:356`). The engine never invokes it. Only the worker knows the PR number. What the engine *does* know is sufficient though: the bead id and the branch `lane/<bead-id>` (`main.rs:193`). Both `gh pr merge` and `gh pr checks` accept a **branch name** as the selector, so the PR is rediscoverable with no new persisted state. This is the cheapest available seam and RESEARCH recommends ARCHITECTURE take it rather than recording PR numbers on beads. Confidence high.

**No config surface exists.** `main()` (`:18-49`) matches on `args.first()` only; `run` takes one optional positional path. Nothing anywhere reads a config file — the only `.beads/config.yaml` belongs to `br` (`br config path`). S1's per-repo flag has no home today. Three candidate shapes: a CLI flag on `run`; a new abacus-owned config file; or a marker in br's config. The third is a layering violation worth naming explicitly — abacus policy stored in the tracker's config namespace. Confidence high.

**`capture()` discards the exit code** (`src/main.rs:311-329`). It folds every non-zero status into a single `String` error carrying stderr. But `gh pr checks` distinguishes **pending (exit 8)** from **failed (exit 1)** by exit code alone. The S7 gate therefore *cannot be expressed* through the current process seam — it needs an exit-code-returning variant of `capture()` or a sibling function. This is the single most concrete engine change the epic implies. Confidence high.

**Attachment point.** Two viable shapes, both for ARCHITECTURE:

- *Inside `cmd_run`'s `Completed` arm* (`:287-291`). Simplest, but couples merge latency to dispatch: no next lane starts until the previous merge's CI finishes. Since CI is minutes and lanes are minutes, this roughly halves throughput and sits badly with S4's "the run continues with other work."
- *A separate subcommand* (`abacus land` / `abacus merge`) looping over closed-beads-with-open-PRs. Decouples merge from dispatch, matches "serialized merges while lanes keep finishing", and is independently testable. Candidate enumeration is cheap: `br list --status closed --json` (confirmed: `-s/--status` repeatable, `--json` includes `labels`, `notes`, `closed_at`, `close_reason`) cross-referenced against `gh pr list --state open --json headRefName`.

RESEARCH leans to the subcommand. Confidence medium — it depends on whether the multi-bead loop lands in this epic.

### CI findings

**State: greenfield.** No `.github/` directory. `gh api repos/DylanDelliColli/abacus/actions/workflows` → `{"total_count":0,"workflows":[]}`. Actions is enabled: `{"enabled":true,"allowed_actions":"all","sha_pinning_required":false}`. Confidence high.

**Toolchain: pin stable.** `Cargo.toml` declares `edition = "2024"` (requires Rust ≥ 1.85) with no `rust-version` and no `rust-toolchain` file. Local active toolchain is stable 1.97.1; nightly is installed but abacus does not need it — nightly was a **`br` build** requirement, and `br` is consumed as a prebuilt binary, not a cargo dependency (`Cargo.toml` deps are only `serde` and `serde_json`). Recommend adding `rust-version` to `Cargo.toml` as the single source of truth the workflow reads. Confidence high.

**Wall-clock estimate.** Measured locally against an empty target dir: `cargo build` 6.2s wall / 12.7s user; `cargo test` 6.5s wall / 25.0s user at ~6 cores. GitHub-hosted `ubuntu-latest` is 4 vCPU and slower per core. Extrapolating: **cold ≈ 1.5-3 min** for checkout + build + test + clippy + fmt; **cached ≈ 45-90s**. Confidence medium — extrapolation, not an Actions measurement. Verify by landing the workflow and reading the first two run durations.

**The blocker: 17 of 45 tests cannot run in CI.** `tests/br_roundtrip.rs:35` `.expect("br must be on PATH")` and `:62` `panic!("{program} must be on PATH")`; `tests/shim.rs` calls `br init` in `init_plain_checkout`. No `#[ignore]`, no skip guard — absence of `br` is a panic, not a skip. Measured split by suite:

| Suite | Tests | Time | Needs `br`? |
|---|---|---|---|
| lib units | 17 | 0.00s | no |
| bin units | 7 | 0.00s | no |
| `br_roundtrip` | 15 | 4.56s | **yes** |
| `shim` | 2 | 0.30s | **yes** |
| `merge_jsonl` | 2 | 0.00s | no |
| `version` | 2 | 0.00s | no |

CI-portable today: **28 tests, ~0.2s.** Blocked: **17 tests, 4.86s** — essentially all the integration coverage. Confidence high.

**`br` has no CI install recipe.** It is a stripped 24MB prebuilt ELF at `~/.local/bin/br`, version 0.3.2, pinned to **untagged** upstream commit `5154a379`, and no local `br` source repo exists. Building it requires nightly. Three options for S8, none free:

1. Gate the br-requiring tests out of CI. Cheapest — but then the remote leg validates strictly *less* than the local leg, which undercuts S7's premise that CI is the canonical crash-surviving evidence. If taken, S7's wording should be adjusted honestly rather than left implying parity.
2. Publish `br` as a release artifact or package and download it in the workflow. Restores parity; needs upstream cooperation the operator routes.
3. Self-hosted runner on the operator's machine. Restores parity but dies with the host, defeating the crash-survival rationale outright.

This is the highest-risk unknown in the epic. Confidence high on the constraint; it needs an operator decision.

Compounding it: `bin/br-shim:5` hardcodes `real_br=/home/ddc/.local/bin/br`, so `tests/shim.rs` cannot pass on any machine but this one even if `br` were installed elsewhere.

**Token scope gotcha.** `gh auth status`: the active account holds `admin:org, admin:public_key, repo` — **no `workflow` scope**. Pushing `.github/workflows/*.yml` over the SSH remote is unaffected (SSH uses the key, verified as authenticating correctly), but any path that writes a workflow file through `gh api` or an HTTPS remote using gh's credential helper fails with a scope error. Worth stating in the S8 bead so a worker does not burn a cycle on it. Confidence high on the scope fact, medium on the SSH-is-unaffected inference.

**Standard workflow shape.** `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`, on `pull_request` and `push` to `main`. That trio is already the repo's de-facto gate — it appears verbatim in worker evidence (PR 17 body; `ab-mk9` and `ab-nl5.2` bead notes). Adopting it in CI creates no new contract, only enforcement.

### Conflict-resolution option space

Laid out with tradeoffs; no recommendation, per the substage contract.

**Branch-update mechanism** determines which conflict-resolution options are even reachable:

| Mechanism | Conflict hook? | CI effect | Review/force-push effect |
|---|---|---|---|
| `gh pr update-branch` (merge, default) | **None** — fails on conflict | New head SHA, CI re-runs | No force-push; review state preserved |
| `gh pr update-branch --rebase` | **None** — fails on conflict | New SHAs, CI re-runs; in-flight runs orphaned | Rewrites branch; equivalent to force-push |
| Local fetch + `git merge origin/main` + push | **Yes** — working tree with markers | New head SHA, CI re-runs | No force-push |
| Local fetch + rebase + `push --force-with-lease` | Yes | Orphans in-flight CI | Force-push; invalidates review state |

**S6 forces the local path.** Neither `gh pr update-branch` variant gives the engine anywhere to stand when a conflict occurs — it just fails. Resolving requires a working tree. And **the original lane worktree is gone by then**: `should_reap_lane(Completed)` is true (`lib.rs:98`), and a closed bead is exactly the `Completed` state, so the checkout is destroyed before the PR is ever merge-eligible. The merge phase needs its own worktree — `git worktree add`, or `herdr worktree open --branch <lane-branch>`. Confidence high; hard constraint on S6.

Also note the repo setting `allow_update_branch: false`. Whether it gates the update-branch API or only the UI affordance is **unverified**. Verify on a live open PR: `gh pr update-branch <n>` and read the error.

**Resolution approaches:**

- **Mechanical, domain-specific** (the `merge-jsonl` pattern). Resolve file classes whose semantics permit it; fail loudly otherwise. Narrow but safe, precedented. Source code is not a class with mechanical semantics.
- **`git rerere`.** Replays prior resolutions; fits the queue's repeated-hunk pattern; cannot produce a first resolution. Silent-wrong risk on superficially similar conflicts.
- **Strategy options (`-X ours`/`-X theirs`).** Always terminate, never fail — that is the danger: a green tree manufactured by discarding a side, invisible to S2 when the discarded hunk's test went with it.
- **GitHub merge queue.** Doesn't resolve; dequeues. Covered under Prior art.
- **Agent-dispatched resolution lane.** A fresh herdr lane on the conflicted branch. Needs: a worktree (original reaped); a prompt carrying bead id AND resolution-attempt framing (CONSTRAINTS finding 3 applies); the S2 validation loop as exit condition; a bound on attempts before S4's park. The authoring worker is gone — the resolver has only the diff and the bead.
- **Park immediately** (planner's original recommendation, overridden by operator decision 4). Retained as the fallback inside every approach above.

### Repo-agnostic audit

| Abacus-specific today | Location | Generalizing seam |
|---|---|---|
| Real `br` path hardcoded | `bin/br-shim:5` `real_br=/home/ddc/.local/bin/br` | `BR_REAL` env override, or PATH search excluding self |
| Operator worktree paths in fixtures | `src/lib.rs:229`, `:325` | Test-only; parameterize the fixture |
| **`--base main` hardcoded in the worker prompt** | `src/lib.rs:190` (`gh pr create --base main`) | Read `defaultBranchRef` from gh, or a config value. A repo on `master`/`develop` breaks today |
| Agent kind fixed to codex | `src/main.rs:226` (`"--kind", "codex"`) | Config or CLI flag; relevant since S9 promises any repository |
| Tracker fixed to `br` in prompt text | `src/lib.rs:182-192` | Acceptable — `br` is pinned substrate per NORTH-STAR, not a seam |
| **Repo identity: absent entirely** | `cmd_run` takes a filesystem path only; nothing reads `git remote` | Pass `cwd: Some(&repo)` through `capture()` (already the pattern for `br`) and let gh infer, or add `-R OWNER/REPO` |

**Already generalized, no work needed:** the worktree layout `~/.herdr/worktrees/<repo>/` is never constructed by abacus — `parse_worktree_created` (`lib.rs:147-175`) *reads* `result.worktree.path` back out of herdr's JSON. Layout is herdr's concern. Likewise the branch grammar `lane/<bead-id>` (`main.rs:193`) is engine-owned and portable, and `OPERATOR_SEAT_LABEL` (`lib.rs:7`) travels. Confidence high.

### Protocol guarantees and races

**The guarantee.** ADR 0002 (`docs/adr/0002:70-74`), `AGENTS.md:78-81`, and `dispatch_prompt` (`lib.rs:190-192`) all state: close is the worker's last act, after push and after PR. It is **test-enforced** — `lib.rs:371-378` asserts `push < pr < close` by string position. So **bead closed ⇒ push succeeded and a PR exists**. That is the merge queue's admission predicate, and it is the strongest thing the epic has going for it. Confidence high.

**Races that remain:**

1. **Close without a PR.** The ordering is prompt text, not enforcement. A worker that closes anyway, a silently-failed `gh pr create`, or the "treat that existing PR as success" clause (`lib.rs:190`) firing on a *closed* PR all yield a closed bead with no open PR. The queue must treat this as a normal skip, not an error.
2. **Closed bead, PR still materializing.** The probe can observe `closed` within the same second the worker created the PR. A queue polling immediately may find nothing. Precedent for the fix: `retry_probe_once` (`main.rs:152`).
3. **The probe-retry shape from PR 17.** `retry_probe_once` retries on **any** error, not just transient ones, and `Err(_)` **discards the first error** — for a CI wait, where the first error is often the informative one, the diagnosis is lost. Do not copy blindly.
4. **Worker pushes after close.** Nothing prevents it. `gh pr merge --match-head-commit <SHA>` is the defense — validate at SHA X, merge with `--match-head-commit X`, and a racing push fails the merge rather than landing unvalidated code. This is the compare-and-swap primitive the queue needs; it exists in gh 2.87.3.
5. **The reap precedes eligibility.** `should_reap_lane(Completed)` destroys the lane checkout exactly when the bead becomes queue-eligible — constrains S6.

**CONSTRAINTS.md bearing on a merge loop:**

- **Finding 4, crash-first-class** (`CONSTRAINTS.md:44-49`) is the binding one. PR state, branch, and bead status are all durable off-host, so the queue is *nearly* reconstructible from GitHub + the br store alone. The exception: a local working tree mid-conflict-resolution — the one piece of state with no off-host copy. Design implication: never let an uncommitted worktree hold the only copy of a resolution; push resolution commits to the PR branch as they are made.
- **Finding 3, launch env carries bead and attempt** (`:26-32`) applies to an S6 resolution lane — it is a worker launch and inherits the requirement, including "attempt".
- **Finding 1, `br` not `bd`** (`:13-18`) — the queue reads bead state; keep using `br`.
- **Finding 2, provider identity per execution** (`:20-24`) — engages only if S6 dispatches an agent.

### Pitfalls and open unknowns

**`gh pr checks` conflates "no CI" with "CI failed" — probed.** On PR 17 (zero workflows): prints `no checks reported`, **exit 1**; `--json` and `--required` do not change this. Exit 1 means *failed or absent*; exit 8 means pending. **The disambiguation seam is `gh pr view <n> --json statusCheckRollup`** — returns `[]` and exit 0 for the no-checks case. RESEARCH recommends that as the gate primitive rather than `gh pr checks` exit codes. This also makes S8's "eligibility requires CI present" a *mechanical* necessity, not merely policy — without it the gate cannot distinguish a clean repo from a broken one. Confidence high.

**`gh pr merge --auto` is unavailable today.** `allow_auto_merge: false` on the repo (probed). It would also delegate ordering to GitHub, contradicting S3. `--admin` bypasses requirements and must never be used by the queue — it would let a red PR land. Confidence high.

**Merge method precedent.** All 17 PRs merged as merge commits (`git cat-file -p e758a96` shows two parents, committer `GitHub`; `viewerDefaultMergeMethod: MERGE`). Using `-m/--merge` preserves the history shape. Confidence high.

**Lane branches are being deleted by something unidentified.** `delete_branch_on_merge: false`, yet `git ls-remote --heads origin` shows **only `main`**. Worth pinning down — crash recovery may need the branch to still exist. Confidence high on the observation, low on the cause.

**`mergeable` / `mergeStateStatus` are computed lazily.** First query can return UNKNOWN while GitHub computes. A queue that reads once and believes the answer will misjudge; poll-until-known. Confidence medium — probed only on a merged PR, which is weak evidence. Verify on a live open PR, two reads a few seconds apart.

**Force-push after rebase** orphans in-flight CI and invalidates review state; merge-mode update avoids both. If ARCHITECTURE picks rebase for a clean history, it buys that cost knowingly.

**Serialized merge latency vs lane throughput.** N queued PRs cost at minimum N serialized CI runs. At 1.5-3 min cold, a 3-bead drain adds ~5-9 min of merge-gate wall clock — comfortable. At N=20 it is 30-60 min of serialized gating while lanes keep finishing — where the attachment-point decision starts to matter greatly.

**The moving-base hazard has not actually been measured in this repo.** Main's inter-PR commits are overwhelmingly tracker-only (`backlog:` commits touching `.beads/issues.jsonl`), and lane branches never touch `.beads` (ADR 0002). PR 17's only main-side commit between branch point and merge was a backlog commit; the shift report *predicted* a `src/main.rs` conflict that did not materialize. Observed cross-lane conflict rate to date: approximately zero. The epic de-risks an anticipated hazard, not a measured one — S3 changes the base-movement regime by design, and one PR is not a sample, but the frame should say so honestly. Confidence high; worth an explicit operator sentence at ARCHITECTURE.

**Host crash mid-merge-phase.** Reconstructible from GitHub + br store: open PRs, bead↔branch mapping, head SHAs, check state, landed merges. NOT reconstructible: an in-progress local conflict resolution, and any in-memory queue position. Both should be designed as recomputable rather than persisted.

**API rate limits are a non-issue.** `core` and `graphql` both 5000/hr, nearly untouched. A queue polling at 10s for 3 min per PR spends ~18 calls per PR. Confidence high.

**Open unknowns requiring an operator decision or a live probe:**

1. Does `br` get into CI, or does CI validate a strict subset? (Operator decision; blocks S7/S8.)
2. Is the multi-bead loop in this epic or a prerequisite? (Operator decision; blocks the success metric.)
3. Does `allow_update_branch: false` gate the update-branch API? (Live probe on an open PR.)
4. Who deletes merged lane branches? (Live probe.)
5. In-loop merge phase vs separate subcommand? (ARCHITECTURE.)
6. Adopt or reject GitHub's native merge queue? (ARCHITECTURE; it is an alternative to the epic.)

### Provisional module fingerprints

All PROVISIONAL.

| Path | Symbols | Seam | Verify | Confidence |
|---|---|---|---|---|
| `src/main.rs` | `cmd_run` (`:177`), `capture` (`:311`) | The dispatch spine and the sole process-spawn seam. Every merge-phase shell-out lands here; `capture` needs an exit-code-aware sibling for the S7 gate | `grep -n "fn cmd_run\|fn capture" src/main.rs` | high |
| `src/main.rs` | `main` (`:18`), `usage` (`:51`) | The CLI surface. S1's flag and any `abacus land` subcommand enter here | `grep -n "fn usage" src/main.rs` | high |
| `src/main.rs` | `retry_probe_once` (`:152`), `probe_bead_outcome` (`:166`) | The retry idiom a CI-wait loop would reuse — note it discards the first error | `grep -n "fn retry_probe_once" src/main.rs` | high |
| `src/lib.rs` | `dispatch_prompt` (`:180`) | Worker protocol text; carries the hardcoded `--base main` and the close-last ordering the queue depends on | `grep -n "gh pr create --base main" src/lib.rs` | high |
| `src/lib.rs` | `BeadOutcome` (`:79`), `classify_bead_status` (`:86`), `should_reap_lane` (`:98`) | Outcome classification. A merge phase adds states (validated / parked / merged) that may or may not belong in this enum | `grep -n "enum BeadOutcome" src/lib.rs` | medium |
| `src/lib.rs` | `parse_ready` (`:39`), `select_bead` (`:45`), `ReadyBead` (`:26`) | br JSON parsing. Queue candidate enumeration needs a sibling parser — the `br list --json` envelope is `{"issues":[…],"total":…}`, not a bare array | `br list --status closed --json \| head -c 200` | high |
| **new** `src/` module | — | gh JSON parsing + merge policy. Pure, fixture-testable, mirrors how `lib.rs` isolates parsing from spawning | n/a — does not exist | medium |
| `tests/br_roundtrip.rs` | fake-CLI harness (`:249-306`) | **The key test seam.** Already writes a fake `herdr` script into a temp dir, PATH-shadows it, logs calls. A fake `gh` follows the identical pattern — the merge queue is integration-testable without touching GitHub | `sed -n '249,306p' tests/br_roundtrip.rs` | high |
| `tests/br_roundtrip.rs` | `br` (`:31`), `find_on_path` (`:58`) | The CI-portability blocker: hard panic when `br` is absent | `PATH=/usr/bin:/bin cargo test --test br_roundtrip` | high |
| `bin/br-shim` | `real_br` (`:5`) | Hardcoded operator path; blocks S9 generality and CI for `tests/shim.rs` | `grep -n real_br bin/br-shim` | high |
| **new** `.github/workflows/ci.yml` | — | S8 deliverable | `gh api repos/DylanDelliColli/abacus/actions/workflows` | high |
| `Cargo.toml` | `[package]` | Toolchain pin (`rust-version`) for the workflow to read | `grep -n "edition\|rust-version" Cargo.toml` | high |
| `AGENTS.md` | Lanes section (`:72-84`) | Contains "Autonomy ends at the PR. Never merge to `main`" — must be amended for auto-merge mode | `grep -n "Autonomy ends at the PR" AGENTS.md` | high |
| `NORTH-STAR.md` | Non-goals (`:47-63`), Success condition (`:36-45`) | The RECORD-gate amendment target | `grep -n "Merging to main" NORTH-STAR.md` | high |

### Provisional bundle groups

All PROVISIONAL — DECOMPOSITION re-derives footprints from the locked architecture and drops candidates that do not survive.

**Group A — engine spine.** Anything touching `cmd_run`'s phase sequence or `capture`'s signature. Predicted members: the exit-code-aware `capture` variant, the merge-phase attachment, and the multi-bead loop if it lands in this epic. All three edit overlapping regions of `src/main.rs`, and `capture`'s signature change ripples to every call site. **One lane, one PR.** Confidence high.

**Group B — CLI and config surface.** `main()`'s arg match, `usage()`, and the S1 flag. Overlaps Group A in `src/main.rs`'s top region. Merge into A unless ARCHITECTURE puts the merge phase in a separate subcommand, in which case B is the subcommand's own entry point and can stand alone. Confidence medium.

**Group C — worker protocol text.** `dispatch_prompt` plus its assertion block (`lib.rs:348-379`) plus `AGENTS.md`'s "Autonomy ends at the PR". Predicted members: the `--base main` de-hardcoding, any auto-merge-mode prompt change, and the AGENTS.md amendment. The tests assert prompt substrings verbatim, so any two prompt edits in separate lanes conflict. **One lane.** Confidence high.

**Group D — CI groundwork (S7/S8).** `.github/workflows/ci.yml`, `Cargo.toml`'s `rust-version`, and the test-portability fix (skip guards in `tests/br_roundtrip.rs` and `tests/shim.rs`, plus `bin/br-shim`'s path). Footprints barely overlap but are **sequentially dependent**: the workflow is meaningless until the suite can run without `br`. Model as a dependency chain, or one lane if the operator picks the gate-tests-out option. Confidence medium — shape depends on open unknown 1.

**Group E — conflict resolution (S6).** A new module plus a merge-phase call site in Group A's territory. Overlaps A on the call site only. Given S6 is the largest single unknown, probably its own lane sequenced **after** A lands. Confidence low — shape depends entirely on which resolution approach ARCHITECTURE picks.

**Group F — repo-agnostic cleanup (S9).** `bin/br-shim`'s path (shared with D), `lib.rs` fixture paths, and the agent-kind flag (shared with B). Genuinely scattered; a candidate for one "generalization sweep" lane, but only **after** A/B/C, since it edits the same files. Confidence medium.

Cross-cutting: **Groups A, B, C, and F all touch `src/main.rs` or `src/lib.rs`.** Sequencing matters more than bundling — dispatching any two as concurrent lanes is the scenario that would finally produce the cross-lane source conflict this repo has not yet measured. This epic's own decomposition is the most likely first real test of the moving-base hazard it exists to solve.

---

## ARCHITECTURE

Produced 2026-08-15 by the orchestrator inline. **Material producer
substitution:** the default producer (gaudi skill) presumes an existing
module to audit or an existing bead tree to gate; this substage locks
greenfield contracts from the RESEARCH base, so the orchestrator
produced it directly, applying gaudi's interface-coherence lens — every
decision below names its seam and its smell risk where one exists.

### Decisions taken at the RESEARCH gate (operator, 2026-08-15)

- **The multi-bead loop is in this epic.** The success metric stands as
  framed. Affects bundle group A.
- **CI validates the portable subset** (28 tests + clippy + fmt), not
  the full suite. The `br`-requiring 17 integration tests stay
  local-only. **S7 is reworded accordingly:** the local leg is the
  full-parity gate; the remote leg is the crash-surviving, GitHub-
  visible gate over the portable subset. S8's "eligibility requires CI
  present" stands — RESEARCH showed it is a mechanical necessity, not
  just policy.

### Changed research assumptions

RESEARCH's provisional lean toward recording nothing and its subcommand
lean are confirmed below (D2, D4); its assumption that the loop might be
out of scope is resolved (in scope); its S7-parity concern is resolved
by the operator's subset decision rather than by installing `br` in CI.

### Locked decisions

**D1 — Engine-owned serialized queue; GitHub's native merge queue is
rejected.** Three disqualifiers from RESEARCH stand: it validates a
speculative merge commit the local leg cannot run against (breaks S2);
it owns ordering (displaces S3); it dequeues conflicts instead of
resolving them (no S6 hook). It also requires branch-protection
machinery this repo does not have.

**D2 — The merge queue is a separate subcommand: `abacus land`.**
Decoupled from dispatch so merges serialize while lanes keep finishing
(S3, S4). `abacus land [repo]` loops: enumerate candidates (open PRs on
`lane/*` branches whose bead is closed — `gh pr list --state open
--json headRefName` intersected with `br` status), process one at a
time, poll between rounds. `--once` processes the current candidate set
and exits (the integration-test entry point). Termination: continuous
until interrupted — the overnight pattern is drain processes plus one
land process per repo. **Sole-merger assumption, stated:** during a
run, `abacus land` is the only writer to the repo's main. Before each
merge it re-reads `mergeStateStatus`; a `BEHIND` result (external push
to main) loops back to update-and-revalidate rather than merging.

**D3 — The multi-bead dispatch loop is `abacus drain`:** a thin loop
over the existing single-bead cycle — while label-eligible ready beads
exist, run one dispatch cycle to settle and reap, then select again.
Lane concurrency comes from running multiple drain processes (the
pattern the ten-worker pilot validated), not from in-process lane
management. **Claim-race defense:** a failed or already-taken claim is
a normal event — reselect the next ready bead, never abort. This makes
claim atomicity a non-blocking property: either `br`'s claim is guarded
(race resolves cleanly) or the reselect absorbs it.

**D4 — PR discovery is by branch name.** The engine already knows
`lane/<bead-id>`; both `gh pr merge` and `gh pr view` accept a branch
selector. No PR numbers are persisted on beads; no new state.

**D5 — The gate primitive is `gh pr view --json
statusCheckRollup,mergeable,mergeStateStatus`,** polled until known,
never `gh pr checks` exit codes (which conflate failed with absent). An
empty `statusCheckRollup` on an eligibility check means "no CI" and the
repo is refused (S8). Engine change: a new exit-code-aware sibling of
`capture()` (`capture_status`, returning code + stdout + stderr);
`capture()` itself is untouched, so its eight call sites do not churn.

**D6 — Branch update is a local merge of origin/main into the PR
branch,** performed in a land-owned plain `git worktree` (the lane
worktree is already reaped; no agent is needed for the mechanical
path, so herdr is not involved here). No force-push ever: rebase and
`gh pr update-branch` are rejected (the former orphans CI and rewrites
history, the latter has no conflict hook — which also moots RESEARCH
open unknown 3). Merge-commit history matches all 17 merged PRs.

**D7 — Validation legs.** Local leg: full suite + clippy + fmt run in
the landing worktree on the updated branch — full parity including the
`br`-requiring integration tests, because it runs on the operator host.
Remote leg: CI green on the exact validated head SHA — the portable
subset. Order: update → local leg (fails in seconds) → push → CI wait →
merge. Priced against the 30s suite budget: the local leg is one full
suite run per landing, ~5.1s at current measure.

**D8 — S6 conflict resolution is layered, cheapest first:**
1. Domain drivers (the `merge-jsonl` precedent) resolve file classes
   with mechanical semantics; they already fail loudly into a normal
   conflict otherwise.
2. `git rerere` is enabled in the landing worktree as an accelerator —
   replaying resolutions the layers below produced earlier in the same
   queue drain.
3. One bounded agent-resolution attempt: a herdr lane opened on the
   conflicted branch (`herdr worktree open --branch`), prompt carrying
   the bead id, the attempt marker, and explicit
   this-is-a-resolution-not-implementation framing (CONSTRAINTS
   findings 2 and 3 apply — it is a worker launch). Exit condition is
   the D7 local leg.
4. Anything still unresolved — or a resolution that fails validation —
   parks per S4.
Crash constraint (CONSTRAINTS finding 4): resolution commits are pushed
to the PR branch as they are made; an uncommitted landing worktree
never holds the only copy of anything.

**D9 — Merge and park mechanics.** Merge: `gh pr merge --merge
--match-head-commit <validated SHA>` — the compare-and-swap that makes
a worker pushing after close a merge *failure* (loop back to update)
instead of an unvalidated landing. After a successful merge the remote
branch is deleted (matches the observed hygiene of all 17 merged PRs;
safe, the SHA is in main; resolves RESEARCH open unknown 4 by making
deletion an owned act instead of a mystery). Park: a `gh pr comment`
carrying the failure evidence — off-host durable, read at morning
review. The bead stays closed; no tracker writes on park.

**D10 — S1 opt-in is the act of running `abacus land` on a repo.**
Within operator decision 3's "configuration or invocation": invocation.
No config file until observed need (MVP-first ruling); `abacus land`
refuses a repo whose eligibility check finds no CI (D5). The morning-
review default (S5) is therefore literal: not running land changes
nothing.

**D11 — New module `src/land.rs`:** pure policy — gh JSON parsing, a
`LandOutcome` type, candidate selection, gate decisions — separate from
process-spawning, mirroring how `lib.rs` isolates parsing today.
`BeadOutcome` is untouched: landing states are not bead-outcome states,
and growing that enum was the named smell risk.

**D12 — Crash recovery is stateless recomputation.** The queue is
re-derived on every land start from GitHub (open `lane/*` PRs) and the
`br` store (closed beads); worktrees are disposable and recreated;
queue position is never persisted. With D8's push-as-you-resolve, the
only unrecoverable state class RESEARCH identified is closed.

**D13 — AGENTS.md amendment (Group C):** the worker contract is
unchanged — workers still never merge. The line "Autonomy ends at the
PR. Never merge to `main`" gains the engine-side exception: in land
mode the *engine* merges validated PRs. Prompt text does not change,
so the verbatim-substring test coupling is not disturbed by this
epic's protocol edit.

### Smell and migration risks

- `capture` signature ripple — avoided by D5's sibling function.
- `BeadOutcome` enum growth — avoided by D11's separate type.
- Prompt-assertion coupling (`lib.rs` verbatim substring tests) — Group
  C stays one lane; D13 avoids prompt edits entirely.
- Sequencing over bundling: groups A, B, C, F all touch
  `src/main.rs`/`src/lib.rs`; DECOMPOSITION must express order as
  dependencies, and this epic's own drain is the first real test of
  the moving-base regime it builds for (RESEARCH's calibration note:
  the hazard is anticipated, not yet measured — the operator accepts
  building ahead of measurement here by design, since S3 changes the
  regime).

### RESEARCH open unknowns, resolved

1. `br` in CI → operator: portable subset (gate decision above).
2. Loop scope → operator: in this epic (D3).
3. `allow_update_branch` API behavior → mooted by D6 (no update-branch API).
4. Branch deletion cause → superseded by D9 (deletion becomes owned).
5. Attachment point → D2 (subcommand).
6. Native merge queue → D1 (rejected).

Remaining verify-at-implementation items (not open questions): live-PR
`mergeable` poll behavior (first land integration test observes it),
actual CI durations (first two workflow runs), claim behavior under
concurrent drains (absorbed by D3's reselect defense either way).

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

---

## TEST-STRATEGY

Produced by a columbo-type subagent, 2026-08-15. **Measured baseline: 5.36s warm wall clock** for `cargo test` (45 tests: lib units 17 @ 0.00s, bin units 7 @ 0.00s, `br_roundtrip` 15 @ 5.08s, `merge_jsonl` 2 @ 0.00s, `shim` 2 @ 0.18s, `version` 2 @ 0.00s — `br_roundtrip`'s real-`br` calls are 95% of the suite). **Remaining budget: 24.64s** against `FULL_SUITE_WALL_CLOCK_BUDGET_SECONDS = 30`. Every runtime in the matrix below is **ESTIMATED**; only the 5.36s baseline is measured.

Three probes were run to make the fixture assertions concrete rather than guessed, and their outputs are the fixture literals the matrix cites:

- `gh pr view 17 --json statusCheckRollup,mergeable,mergeStateStatus,headRefName,headRefOid` → `{"headRefName":"lane/ab-mk9","headRefOid":"3f19f98d...","mergeStateStatus":"UNKNOWN","mergeable":"UNKNOWN","statusCheckRollup":[]}`, **exit 0**. Confirms D5's primitive: the no-CI case is an empty array at exit 0, unlike `gh pr checks`'s exit 1.
- `gh pr view 14148 --repo cli/cli --json statusCheckRollup,...` → `mergeable: MERGEABLE`, `mergeStateStatus: BLOCKED`, 18 rollup entries shaped `{"__typename":"CheckRun","name":"lint","status":"COMPLETED","conclusion":"FAILURE","workflowName":"Lint",...}`. This is the populated-rollup fixture shape.
- `br list --status closed --json` → `{"issues":[…],"total":…}`. Confirms RESEARCH: the envelope differs from `br ready --json`, so a sibling parser cannot copy `parse_ready`.

**Layering rationale.** Gate policy is pure and goes in `src/land.rs` units over JSON fixtures — cheap, exhaustive, and where the branch explosion (rollup states × mergeable states × local-leg result) belongs. Integration coverage is placed only at seams where two systems meet and the unit layer is structurally unable to prove the claim: **SHA identity between what was validated and what was merged** (only real git produces that), **ordering and base-movement across two landings** (only a real repo has a post-merge main), and **the process-level negative space** (a call log is the only place "`pr merge` never happened" can be observed). Integration tests use the fake-CLI harness at `tests/br_roundtrip.rs:249-306` extended to a fake `gh` and a fake `br` with real `git` — CI-portable. Two tests need real `br` and are local-only, and are marked as such.

**New files:** `tests/land.rs` (fake gh + fake br + real git, CI-portable) and `tests/drain.rs` (fake herdr + fake br, CI-portable). New land tests do **not** go in `tests/br_roundtrip.rs`, because that binary's `br()` helper panics when `br` is absent (`:35`, `:62`) and would drag CI-portable coverage into the local-only set.

### Story-by-test matrix

| ID | Story / decision | Layer | Extends or new | Assertion (concrete) | Est. |
|---|---|---|---|---|---|
| T1 | S1, D5 | unit | new — `src/land.rs` | `{"statusCheckRollup":[],"mergeable":"MERGEABLE","mergeStateStatus":"CLEAN"}` → `CiState::Absent`, and `Absent != Red` | 0.00s |
| T2 | S1, S8, D5, D10 | unit | new — `src/land.rs` | Eligibility on `Absent` → refusal whose reason contains "no CI"; eligibility on a `FAILURE` rollup → `Park`, **not** refusal. The pairing is what proves the distinction is real | 0.00s |
| T3 | S1, S8, D10 | integration | new — `tests/land.rs` | `abacus land` against a repo whose candidate PR has `statusCheckRollup: []` exits non-zero with a CI-absence message, and the call log shows **no** `git worktree add`, **no** `git push`, **no** `gh pr merge` — refusal precedes side effects | 0.25s |
| T4 | S2, D6, D7, D9 | integration | new — `tests/land.rs` | Happy cycle, main ahead by one commit: after `abacus land --once`, `git log --format=%P` on the branch head shows a two-parent merge whose second parent is `origin/main`'s tip; the local-leg commands appear in the log before the push; and the single `gh pr merge --merge --match-head-commit <SHA>` call's SHA equals the branch head SHA after the update. **Only test that proves validated SHA and merged SHA are the same object** | 0.45s |
| T5 | S2, S4, D7 | integration | new — `tests/land.rs` | Local leg fails (fake `cargo clippy` shim exits 1) after a clean update → `gh pr comment` posted, **no** `gh pr merge`, and the branch is not pushed for merge. Distinct from T15: this is the local leg, T15 is the remote leg | 0.40s |
| T6 | S2, D6 | unit | new — `src/land.rs` | The update argv builder emits `git merge origin/<default>` with no `-X ours`, no `-X theirs`, no `--strategy-option` for any input | 0.00s |
| T7 | S3, D2 | integration | new — `tests/land.rs` | Two candidates: PR2's update merge commit has PR1's **merge commit** as its second parent, proving PR2 revalidated against the post-merge base, not merely that the two merges were logged in order | 0.60s |
| T8 | S3, D2 | integration | new — `tests/land.rs` | Fake gh reports `CLEAN` at gate time and `BEHIND` at the pre-merge recheck → no merge; the log shows a second update + second CI wait. Makes D2's stated sole-merger assumption safe when violated rather than silently landing stale-validated code | 0.45s |
| T9 | S3, D4 | unit | new — `src/land.rs` | gh gives `lane/ab-a`, `lane/ab-b`, `feature/manual`; br closed gives `ab-a`, `ab-c` → candidates == `[ab-a]`. Non-lane branch excluded; open-bead lane excluded | 0.00s |
| T10 | S3, D4 | unit | new — `src/land.rs` | The real `{"issues":[{"id":"ab-mk9",…}],"total":N}` envelope parses to ids; a bare `[…]` array is an error, so a parser copied from `parse_ready` fails loudly | 0.00s |
| T11 | S3 (RESEARCH race 1) | unit | new — `src/land.rs` | Closed bead `ab-c` with no open PR → enumeration returns `Ok` with `ab-c` absent, **not** `Err` | 0.00s |
| T12 | S3 (RESEARCH race 1) | integration | new — `tests/land.rs` | Same at process level: two closed beads, one PR → exit 0, the PR merges, the PR-less bead does not abort the cycle | 0.30s |
| T13 | success metric, D3 | integration | new — `tests/br_roundtrip.rs` (**local-only**, real `br`) | Three ready beads, fake herdr closes each on prompt → three dispatch cycles run, then `abacus drain` exits 0 on an empty ready set. Nothing else proves the loop terminates or handles more than one bead; the epic's success metric is literally this | 0.70s |
| T14 | D3 | integration | new — `tests/drain.rs` | Fake `br` fails `update <id1> --claim` and succeeds for `<id2>` → lane opens for `id2`, exit 0, no abort. Makes claim atomicity a non-blocking property | 0.35s |
| T15 | S4, S7, D9 | integration | new — `tests/land.rs` | Rollup with a `FAILURE` conclusion → `gh pr comment` carries evidence, `gh pr merge` **never** appears, PR left open, and the fake `br` log contains **no** write verb (`update`/`close`/`label`) — D9's "no tracker writes on park" | 0.40s |
| T16 | S4, D9 | unit | new — `src/land.rs` | Park body for a clippy failure contains the bead id, the head SHA, the string `clippy`, and the captured stderr excerpt. The comment *is* the morning-review evidence; a generic body silently defeats S4 | 0.00s |
| T17 | **S5 (regression)** | integration | **EXTENDS** `abacus_run_reaps_a_clean_lane_without_force_after_the_worker_closes_its_bead`, `tests/br_roundtrip.rs:422` | Add a logging fake `gh` to that test's PATH; assert the gh call log is **empty** after a full `abacus run` cycle. ~3 lines. S5's "default unchanged" is exactly the guarantee that erodes when someone later adds a PR check to the run path | +0.02s |
| T18 | S5, D2, D3 | unit | **EXTENDS** `usage_text_describes_the_run_command`, `src/main.rs:336` | `usage()` names `run`, `land`, and `drain`; `abacus_without_a_command_prints_usage` (`br_roundtrip.rs:943`) passes unchanged | +0.00s |
| T19 | S6, D8 | unit | new — `src/land.rs` | Layer stack records calls: a domain driver that resolves means rerere and the agent are **never** invoked. D8's "cheapest first" is the entire point — dispatching an agent first burns a lane per conflict | 0.00s |
| T20 | S6, D8 | unit | new — `src/land.rs` | After one failed agent attempt the outcome is `Park`; a second dispatch never occurs. Unbounded retry is an overnight-run-consuming loop | 0.00s |
| T21 | S6, S4, D8 | unit | new — `src/land.rs` | A resolution whose local leg fails → `Park`, never `Merge`. S6's explicit clause and the reason the operator's override is safe | 0.00s |
| T22 | S6, D8 layer 1 | integration | new — `tests/land.rs` | Main and branch both append to `.beads/issues.jsonl`; the `merge.beads-jsonl.driver` is configured in the landing worktree → update resolves, local leg runs, merge proceeds, and the fake `herdr` log is **empty**. This is the one conflict class RESEARCH shows actually occurs here (main's inter-PR commits are overwhelmingly tracker-only), so it is the highest-probability real conflict | 0.50s |
| T23 | S6, D8 layers 3-4 | integration | new — `tests/land.rs` | Conflicting `src/main.rs` hunks, no driver, no rerere entry → fake herdr logs `worktree open` + `agent prompt` **exactly once**, the fake agent leaves the conflict unresolved, park comment posted, no merge. Bounds the agent layer at the process level and proves park is reachable from the deepest layer | 0.55s |
| T24 | S6, D8 layer 2 | integration | new — `tests/land.rs` | Two PRs conflicting on the same hunk: the resolution recorded during PR1's landing replays for PR2 with **no** agent dispatch. Marginal — see deferral candidates | 0.60s |
| T25 | S7, D5 | unit | new — `src/land.rs` | Two `CheckRun`s, both `status: COMPLETED` / `conclusion: SUCCESS` → `CiState::Green` | 0.00s |
| T26 | S7, S4, D5 | unit | new — `src/land.rs` | `name:"lint"` FAILURE alongside `name:"test"` SUCCESS → `Red`, and the evidence string contains `lint`. One red among many greens must not average to green, and park evidence must name what failed | 0.00s |
| T27 | S7, D5 | unit | new — `src/land.rs` | One COMPLETED/SUCCESS plus one `{"status":"IN_PROGRESS","conclusion":null}` → `Pending`. This is the exit-8-vs-exit-1 distinction D5 replaces with JSON | 0.00s |
| T28 | S7, D5 | unit | new — `src/land.rs` | A `FAILURE` alongside an `IN_PROGRESS` → `Red`, not `Pending`. Kept separate from T26/T27 deliberately: folding it leaves the precedence rule unproven while both neighbours still pass, and the queue polls for minutes on a doomed head | 0.00s |
| T29 | S7, D5 | unit | new — `src/land.rs` | `SKIPPED` and `NEUTRAL` conclusions alongside SUCCESS → `Green`. A path-filtered skipped job is normal; treating it as red parks every PR forever | 0.00s |
| T30 | S7, D5 | unit | new — `src/land.rs` | `{"__typename":"StatusContext","context":"ci/legacy","state":"SUCCESS"}` mixed with a `CheckRun` → `Green`; `state:"FAILURE"` → `Red`. The rollup is a GraphQL union; a CheckRun-only parser drops legacy statuses or errors on the whole payload. **Verify the exact `StatusContext` field names against a live mixed rollup at implementation** — not probed | 0.00s |
| T31 | S7, D5 | unit | new — `src/land.rs` | The literal PR-17 probe output (`mergeable: "UNKNOWN"`, `mergeStateStatus: "UNKNOWN"`) → `Unknown`; never treated as mergeable. RESEARCH's lazily-computed pitfall | 0.00s |
| T32 | S6, S7, D5 | unit | new — `src/land.rs` | `mergeable: "CONFLICTING"` → `Conflicting`, routing to D8 rather than to merge or park | 0.00s |
| T33 | S7, D5 | integration | new — `tests/land.rs` | Fake gh returns IN_PROGRESS on the first `pr view` and SUCCESS on the second → exactly two `pr view` calls, then a merge; the poll delay is injected, not a real sleep. Proves the wait path terminates into a merge — the most likely false-park in production | 0.35s |
| T34 | S7, D5 | integration | new — `tests/land.rs` | First `pr view` `mergeable: UNKNOWN`, second `MERGEABLE`/`CLEAN` → proceeds; **no merge attempted while UNKNOWN**. This is where ARCHITECTURE's "verify at implementation" item for live-PR poll behavior actually lands | 0.30s |
| T35 | S7, D5 | unit | new — `src/main.rs` bin units | `capture_status("sh", ["-c","printf out; printf err >&2; exit 8"])` → code 8, stdout `out`, stderr `err`. The exit-8-vs-exit-1 distinction `capture()` destroys; RESEARCH's single most concrete engine change | 0.01s |
| T36 | **S5/D5 (regression, CRITICAL)** | unit | new — `src/main.rs` bin units | `capture()` on a non-zero exit still returns `Err` containing the command line and the trimmed stderr. **Nothing currently proves `capture`'s contract** — there is no existing unit test for it — yet D5 promises its eight call sites do not churn. A refactor that unifies the two functions would silently change eight call sites' error strings | 0.01s |
| T37 | S2, S7, D5 | unit | new — `src/land.rs` | Only `(Green, Clean, LocalPass)` yields `Merge`, and the outcome carries the validated head SHA. Given its own test because it is the sole merge-producing path | 0.00s |
| T38 | S2, S4, S7, D5 | unit | new — `src/land.rs` | Row-named table for every non-merge combination: `(Red,Clean,Pass)→Park`, `(Green,Clean,LocalFail)→Park`, `(Pending,Clean,Pass)→Wait`, `(Green,Behind,Pass)→Update`, `(Green,Conflicting,*)→Resolve`. Each row asserted individually with the row name in the failure message | 0.00s |
| T39 | S8 | unit | new — `src/land.rs` or a small `tests/ci_workflow.rs` | `.github/workflows/ci.yml` exists and its toolchain pin string equals `Cargo.toml`'s `rust-version`; the file contains `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --check`. **The pin agreement is the only assertion here with unique value** — the two files drifting apart is a real, silent, recurring failure. Dependency-free string matching; no YAML parser is added to a two-dependency crate for this | 0.00s |
| T40 | S8 | unit | new — `src/land.rs` | The `br`-presence guard predicate returns false for a PATH with no `br`, true for one containing it. Marginal — see deferral candidates | 0.00s |
| T41 | **S9** | integration | new — fixture variation of T8 | One land integration test's fixture repo uses default branch `trunk`, not `main`, proving nothing in the land path hardcodes `main`. Zero marginal cost — it is a fixture parameter, not a new test | +0.00s |
| T42 | **S9** — GATED | unit | **EXTENDS** `dispatch_prompt_carries_bead_identity_and_protocol`, `src/lib.rs:348` | A repo whose default branch is `develop` produces a prompt containing `gh pr create --base develop`. **Gated: ARCHITECTURE locks no decision on the `--base main` hardcode** (`src/lib.rs:190`), which RESEARCH flagged as a live S9 violation. D13 asserts "prompt text does not change" — that holds for the AGENTS.md edit but not for this. DECOMPOSITION needs a decision before this test is authored | +0.00s |
| T43 | **S9** — GATED | integration | **EXTENDS** `tests/shim.rs` | The shim resolves the real `br` via `BR_REAL` (or PATH search excluding self) rather than the hardcoded `/home/ddc/.local/bin/br` (`bin/br-shim:5`), proven by pointing it at a fake `br`. **Gated on the same missing decision**; RESEARCH notes the shim "cannot pass on any machine but this one", which is both an S9 violation and a CI blocker | +0.10s |
| T44 | D12 | integration | new — `tests/land.rs` | Two candidates where fake gh reports the first already merged → it is not re-merged, the second merges, exit 0; and no state file appears anywhere under the repo. D12's falsifiable halves are "nothing is persisted" and "a partially-drained queue re-derives correctly" | 0.40s |

**Cross-cutting invariant, not a test.** `tests/land.rs` defines `assert_no_forbidden_flags(&gh_log, &git_log)` and every test in the file calls it in teardown. It asserts the gh argv never contains `--admin`, `--auto`, or `update-branch`, and the git argv never contains `--force`, `--force-with-lease`, `-f`, `-X ours`, `-X theirs`, or `--strategy-option`. This is deliberately an invariant applied across all paths rather than one dedicated test — a single test proves the flags are absent from one path, whereas the helper proves it from every path the suite exercises, including ones added later.

### Negative-space cases

What must **not** happen, and where each is proven.

| Must never happen | Proven by | Why it is the assertion that matters |
|---|---|---|
| A red PR merges | T15 (`gh pr merge` absent from the call log on a FAILURE rollup), T26 + T28 (policy: any FAILURE → `Red`, even alongside pending), T38 (`(Red,Clean,Pass)→Park`) | The single most important property in the epic. T26/T28 prove the classifier can't be fooled, T38 proves the decision table routes `Red` away from merge, T15 proves the process actually doesn't call merge |
| `--admin` or `--auto` appears in any gh invocation | `assert_no_forbidden_flags`, run in every `tests/land.rs` test | `--admin` bypasses branch requirements and would let a red PR land; `--auto` delegates ordering to GitHub, contradicting S3 |
| A force-push occurs | `assert_no_forbidden_flags` (`--force`, `--force-with-lease`, `-f`) | D6 rejects rebase precisely because force-push orphans in-flight CI and invalidates review state |
| `gh pr update-branch` is invoked | `assert_no_forbidden_flags` | D6 rejects it — no conflict hook, so S6 would have nowhere to stand |
| `-X ours` / `-X theirs` / `--strategy-option` in the update path | T6 (unit, argv builder) **and** `assert_no_forbidden_flags` (all integration paths) | RESEARCH's highest-severity silent-wrong risk; guarded at both layers because the failure is undetectable by any other test |
| Park writes the tracker | T15 (fake `br` log contains no `update`/`close`/`label`) | D9 says the bead stays closed and park is a `gh pr comment` only. The half of T15 most likely to be dropped under time pressure |
| A merge lands a SHA other than the validated one | T4 (merged SHA == post-update branch head), T8, T12 | The compare-and-swap property. Without it, "unvalidated code cannot land" degrades to "usually doesn't" |
| A worker's post-close push lands unvalidated | T8's sibling case — fake gh's `pr merge` fails with a head-mismatch on the first call, and the log shows a second update + revalidate + a merge at the **new** SHA | RESEARCH race 4. The loop-back converts a race into a retry instead of a bad landing |
| An agent is dispatched before cheaper resolution layers | T19 (rerere and agent never invoked when a domain driver resolves), T22 (fake herdr log empty on a `.beads/issues.jsonl` conflict) | D8's "cheapest first"; dispatching first burns a herdr lane per conflict |
| More than one agent resolution attempt | T20 (policy), T23 (exactly one `agent prompt` in the call log) | An unbounded conflict-resolution retry consumes the overnight run |
| A conflict resolution that fails validation merges | T21 (policy → `Park`), T23 (park comment, no merge) | S6's explicit clause; the case that makes the operator's park-first override safe |
| `abacus run` touches gh at all | T17 (fake gh call log empty across a full run cycle) | S5's "default unchanged". Regression-class: `cmd_run` is existing code this epic modifies |
| Land takes any side effect before refusing a no-CI repo | T3 (no `git worktree add`, no push, no merge in the log) | S8 eligibility as a real gate, not a message printed after the damage |
| A closed bead with no PR aborts the drain | T11 (`Ok`, not `Err`), T12 (exit 0, other candidate still merges) | RESEARCH race 1. A queue that errors here stops the overnight run on a normal event |
| A claim failure aborts the drain | T14 (reselect, exit 0) | D3's defense; without it an overnight run dies at bead 1 |
| Queue state is persisted | T44 (no new file appears under the repo) | D12 claims recovery is recomputation; "nothing is written" is the falsifiable half |

### Budget arithmetic

| Block | Tests | Est. added |
|---|---|---|
| Unit — `src/land.rs` policy + `src/main.rs` capture (T1, T2, T6, T9, T10, T11, T16, T19-T21, T25-T32, T35-T40) | 24 | 0.05s |
| Integration — `tests/land.rs` (T3, T4, T5, T7, T8, T12, T15, T22, T23, T24, T33, T34, T41, T44) | 14 | 5.55s |
| Integration — `tests/drain.rs` (T14) | 1 | 0.35s |
| Integration — `tests/br_roundtrip.rs` (T13 new, T17 extension) | 2 | 0.72s |
| Extensions — `src/lib.rs` (T42), `src/main.rs` units (T18), `tests/shim.rs` (T43) | 3 | 0.10s |
| **Total added (ESTIMATED, serial upper bound)** | **44** | **6.77s** |

**Measured 5.36s + estimated 6.77s = 12.13s against the 30s budget. Headroom: 17.87s. Nothing is dropped.**

Two qualifications. First, 6.77s is a **serial upper bound**: cargo parallelises within a binary, so real wall clock is more likely ~9-10s total. Second, the budget measures run time; two new integration binaries add link time on cold builds — not in the 30s figure, and DECOMPOSITION should not be surprised by it.

Because there is headroom, nothing was cut. If a later substage must trim, the two lowest-value-per-second items are **T24** (rerere replay — an accelerator whose failure mode is slowness, not incorrectness) and **T40** (near-trivial guard predicate). **T13** is the most expensive single test and also the one that proves the epic's success metric — the last candidate for cuts, not the first.

### Deliberately untested

| Item | Disposition | Why |
|---|---|---|
| `.github/workflows/ci.yml` actually running; cold vs cached durations | **[no-test] verify-by-first-run** | A test cannot assert what GitHub's runners do. The acceptance criterion belongs on the S8 bead: first two runs green, durations recorded in bead notes |
| The portable subset genuinely runs without `br` | **[no-test in-suite] — the workflow's own green run is the assertion** | The runner has no `br`; a green `cargo test` there *is* the proof. T40 covers only the guard predicate's logic |
| D13's AGENTS.md amendment | **[no-test], docs-only** | Prose guards rot; a string-absence test fails on any legitimate rewording and teaches a worker to edit the test |
| `allow_update_branch` API behavior; who deletes merged lane branches | **already resolved by ARCHITECTURE** | Mooted by D6; superseded by D9 |

### Tripwire self-check

**1. Folded cases.** Four folds specifically avoided, one accepted:
- T26 / T27 / T28 are three tests, not one — folding precedence into either neighbour leaves the rule unproven while both neighbours still pass.
- T37 / T38 split so the sole merge-producing path has its own named test.
- T11 / T12 split so the no-PR skip is proven at policy and process layers, which fail for different reasons.
- T5 / T15 split so each validation leg's park path is independently exercised.
- **Accepted fold: T4** (update + local leg + push + match-head merge in one test) — the SHA-identity assertion is only meaningful if all four happened in order; mitigated because T5, T15, T8, T34 each isolate one leg's failure.

**2. Deleted tests for live code — zero deletions proposed; one named hazard.** All five touchpoints on existing tests (T17, T18, T42, T43, and `br_roundtrip.rs:943`) are additive extensions. DECOMPOSITION carries this as an explicit constraint: **no existing test is retired by this epic; a worker who finds one "superseded" surfaces it as a finding rather than deleting it.** The concrete hazard: `dispatch_prompt_carries_bead_identity_and_protocol` (`src/lib.rs:348-379`) — its `push < pr < close` positional assertion is the merge queue's entire admission predicate. Any bead touching `dispatch_prompt` must carry an acceptance criterion naming that assertion as preserved.

**3. Thinned assertions — three named risks.**
- **T7** can be weakened to "two merges in order", which a queue validating both against the *pre*-merge base would also satisfy. The ancestry assertion is the only form that proves S3.
- **T15's negative half** (no `pr merge`, no `br` write verb) is the assertion, not the comment.
- **Thinning-by-environment:** the skip guards for the 17 `br`-requiring tests could quietly turn local-only integration into "skipped everywhere". The skip-guard bead's acceptance must read: **on the operator host, `cargo test` still reports 45+ passed, 0 ignored** — the guard skips only when `br` is genuinely absent, and D7's local leg runs the full set.

**Gaps surfaced for DECOMPOSITION.** Two S9 items RESEARCH flagged have no locked ARCHITECTURE decision, so T42 and T43 are gated: the `--base main` hardcode at `src/lib.rs:190`, and `bin/br-shim:5`'s hardcoded `real_br` path (also a CI-portability blocker for `tests/shim.rs`).

---

## ARCHITECTURE — addendum: the GitHub merge queue pivot (2026-08-15)

Adopted by operator decision after the bloat reviewer's
post-clarification update. The enabling fact came from the operator: a
GitHub organization exists (`DylanDelliColli-org`, verified via
`gh api user/orgs`) and the repository is public (verified) — so the
native merge queue is available after a repo transfer. D1's rejection
is reversed with its reasoning corrected on the record: the
`merge_group` commit is exactly the moving-base candidate the remote
leg must validate (the "speculative commit" objection was the weak
leg); ordering delegation is acceptable because a merge limit of one
preserves serial landing; dequeued conflicts route to abacus's
exception handler, which is where S6 now lives.

### Revised decisions

- **D1′ (supersedes D1):** GitHub's merge queue owns ordering,
  candidate construction, remote validation on the `merge_group`
  commit, and the merge itself. Repo configuration: branch protection
  with the portable CI jobs as required checks, a merge-queue ruleset,
  merge limit 1. **Prerequisite (operator act, becomes a
  `seat:operator` bead all implementation children depend on):**
  transfer the repository to the org, then apply that configuration.
- **D2′ (supersedes D2):** `abacus land` shrinks to
  **admission → enqueue → exception watch**. Admission: full local
  validation (suite, clippy, fmt — br-dependent tests included) of the
  PR branch composed with current origin/main in a throwaway, unpushed
  worktree; a composition that conflicts at admission routes to the
  exception handler without enqueueing. Enqueue: the gh merge-queue
  verb (exact flag verified at implementation). Exception watch:
  a PR dequeued by the queue, or conflicting at admission, gets exactly
  one agent-resolution attempt in a fresh lane (launch env carries
  bead id, attempt, resolution framing — CONSTRAINTS findings 2–3),
  then re-enqueue on green re-admission or park. `--once` retained as
  the integration-test entry point.
- **D5′ (supersedes D5):** the pre-merge polling loop (BEHIND recheck,
  poll-until-known before merging) is deleted — GitHub owns it. What
  survives in `src/land.rs`: eligibility parsing (queue configured,
  required checks present — probed at land startup), enqueue-result
  parsing, and queue-state reading for the exception watch.
  `capture_status` (exit-code-aware sibling) is still built.
- **D6′ (supersedes D6):** branch-update-and-push machinery is deleted
  from the happy path. The admission worktree is throwaway and never
  pushed. Force-push, rebase, `update-branch`, and `-X` strategy
  options remain forbidden everywhere.
- **D8′ (supersedes D8):** the resolution ladder collapses to:
  admission-conflict or dequeue → one agent attempt → re-enqueue or
  park. `git rerere` and new domain-driver work are dropped as engine
  layers (bloat cuts 3 and 6 absorbed); the resolution lane naturally
  inherits the repo's git config, including the existing
  `merge-jsonl` driver.
- **D9′ (supersedes D9):** the engine never lands a merge itself —
  enqueueing is the act. The forbidden-flag list revises: `--admin`
  remains forbidden always; queue enqueueing is the mechanism, so
  "never delegate ordering to GitHub" is retired; branch deletion
  machinery is deleted (repo setting `delete_branch_on_merge` is the
  operator's knob if wanted).

### Unchanged

D3 (drain), D4 (branch-name discovery), D10 (opt-in is running land;
eligibility now means queue-configured), D11 (pure `src/land.rs`), D12
(stateless recovery — stronger: the queue itself lives on GitHub), D13
(AGENTS.md exception), D14/D15 (S9 de-hardcodes). D7 is redefined
rather than unchanged: the local leg runs at admission; the remote leg
is CI on the `merge_group`, with `merge_group:` added to the S8
workflow triggers.

### Bloat-review dispositions (both runs, consolidated)

Run 1 (pre-amendment) is superseded by run 2 (post-amendment, fresh
pane). Run 2: cut 1 (remote CI) withdrawn by the reviewer after
operator clarification; cuts 3 (rerere) and 6 (branch deletion)
accepted — absorbed structurally by the pivot; cut 2 (S9 generality)
reaffirmed — the operator decided "both in" at the TEST-STRATEGY gate;
cut 4 (teardown invariant) reaffirmed — the invariant is ~0s string
checks and its value is inherit-by-default coverage of future paths;
cut 5 (`--once`) reaffirmed — it is the test contract's entry point.

### Residual tradeoff, recorded honestly

The br-dependent 17 tests validate at admission against main-at-that-
moment, never on the exact `merge_group` composition. Deferred until an
observed failure (operator MVP-first ruling); revive by installing `br`
in CI (RESEARCH option 2) if that parity is ever declared load-bearing.

### New verify-at-implementation items

The exact gh enqueue verb and its behavior on a queue-required repo;
how dequeue events are observed (poll surface); the `merge_group`
trigger's interaction with required-check naming.

---

## TEST-STRATEGY — delta (merge-queue pivot, D1′–D9′)

Produced by a columbo-type subagent, 2026-08-15, against the approved
matrix and the ARCHITECTURE addendum. Every ID T1–T44 accounted for
exactly once: **14 removed, 14 surviving unchanged, 16 reshaped, 8
added (T45–T52). Net matrix: 38 rows.**

**Probe that changed a conclusion:** `gh pr merge --help` (gh 2.87.3) —
on a queue-required branch, bare `gh pr merge <selector>` with no
strategy flag IS the enqueue verb; auto-merge is enabled implicitly
when checks are pending; and `--admin` is the documented queue bypass.
Consequences: the enqueue-result parser needs BOTH success stdout
shapes (added-to-queue, auto-merge-enabled) or pending-CI admissions
park erroneously (T48); `--admin` is promoted to the most load-bearing
forbidden flag; the enqueue argv is pinned positively (T47).

### Removed (14)

T1, T8, T19, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34 —
all planned-but-unwritten rows whose subjects (rollup classification,
pre-merge polling, BEHIND recheck, CAS-merge race, resolution ladder
ordering, rerere replay) die with the superseded machinery. T32's
routing intent is absorbed by T46/T23 (conflicts now discovered by a
real `git merge` exiting non-zero at admission, not gh JSON). Two live
requirements riding on T33 are re-attached, not lost: the
injected-poll-delay seam moves to T50/T51; poll termination moves to
T44's exit assertion.

### Surviving unchanged (14)

T6, T9, T10, T11, T13, T14, T17, T18, T20, T35, T36, T40, T42, T43.
Non-obvious survivals: T6 (the composition step runs the identical
`git merge origin/<default>` argv — only the destination changed);
T20 (the one-attempt bound is now the only resolution policy — red
branch); T35/T36 (`capture_status` is still built; T36 remains the
only guard on `capture()`'s existing contract). **T42/T43 lose their
GATED marker** — D14/D15 supply the decision they were waiting on.

### Reshaped (16)

T2 (eligibility reads ruleset/required-checks configuration, not
rollups), T3 (refusal on ruleset `[]`, still before any side effect),
T4 (admission-composition happy path: throwaway worktree, local leg on
the composition, exactly one enqueue, PR head SHA unchanged at exit),
T5 (admission red → park, no enqueue), T7 (composition freshness:
admission refetches origin/main between cycles — serial revalidation
itself is GitHub's merge-limit-1 property, not locally assertable),
T12 (enqueue as terminal verb), T15 (exception path: failed resolution
→ evidence comment, no enqueue, no tracker write verbs), T16 (two park
body shapes: admission-red and attempt-exhausted), T21 (Park never
Enqueue), T22 (admission worktree inherits `merge.beads-jsonl.driver`
— now the proof the highest-probability conflict class resolves
without burning an agent lane), T23 (composition conflict → exactly
one agent attempt → park, no enqueue), T37 (sole enqueue-producing
path carries admitted head SHA), T38 (four locally-decidable rows;
Wait/Update rows deleted), T39 (adds `merge_group:` trigger presence —
without it every enqueued PR times out), T41 (re-parented to T4), T44
(idempotency against already-queued/already-merged + clean watch
termination).

### Added (8)

| ID | Story / decision | Layer | New in | Assertion (concrete) | Est. |
|---|---|---|---|---|---|
| T45 | S1, S8, D1′, D5′, D10 | unit | `src/land.rs` | Ruleset `[]` → `Ineligible` naming the queue; queue without required checks → `Ineligible` naming checks; both present → `Eligible`. Positive fixture captured from the org repo after the operator's D1′ act — not authorable before it | 0.00s |
| T46 | S6, D2′, D8′ | unit | `src/land.rs` | Composition `Conflict` → `Resolve`, never `Enqueue`, for every local-leg value including not-run | 0.00s |
| T47 | D2′, D9′ | integration | `tests/land.rs` | Happy admission logs exactly one `gh pr merge`, bare form: branch selector, no strategy flag, no `--admin`, no `--match-head-commit`, no `-d` | 0.30s |
| T48 | D2′, D5′ | unit | `src/land.rs` | Enqueue-result parser: added-to-queue stdout AND auto-merge-enabled stdout both → `Admitted`; non-zero ineligibility → distinct error via `capture_status`. Treating the auto-merge shape as failure would park every pending-CI admission — the common case | 0.00s |
| T49 | D2′, D5′, D12 | unit | `src/land.rs` | Queue-state parser: `Queued`/`Merged`/`Dequeued(reason)`/`Absent` classify distinctly; `Dequeued` carries a non-empty reason for the park body | 0.00s |
| T50 | S6, D2′, D8′ | integration | `tests/land.rs` | Queued→Dequeued across two watch reads → exactly one `worktree open` + `agent prompt`; no second enqueue before the attempt terminates; poll delay injected | 0.45s |
| T51 | S6, D2′, D8′ | integration | `tests/land.rs` | One-attempt bound, green branch: agent resolves, re-admission green → exactly one further enqueue, total agent dispatches across the cycle = 1 | 0.50s |
| T52 | D6′, D12 | integration | `tests/land.rs` | Across a full land cycle, abacus's own git argv contains no `push`, and no admission worktree survives at exit | 0.25s |

### Revised forbidden-flags invariant (D9′)

Stays: **`--admin`** (now the one-flag bypass of the entire epic —
gh's documented queue bypass), `update-branch`, git `--force`/
`--force-with-lease`/`-f`, git `-X ours`/`-X theirs`/
`--strategy-option` (still double-guarded by T6). Newly forbidden:
**`--match-head-commit`** (inverted — its presence means abacus is
landing directly instead of enqueueing), **`-d`/`--delete-branch`**,
**`git push` in abacus's own argv** (resolution commits are pushed by
the agent lane, not abacus — lock at DECOMPOSITION), **mutating
`gh api` calls to rulesets/branch protection** (repo configuration is
the operator's prerequisite act, never the engine's). Retired:
`--auto` (ordering delegation is now the mechanism; T47's positive
argv pin is the better guard) and the blanket `gh pr merge` ban
(replaced by the shape rule: bare enqueue form only).

### Revised budget arithmetic

6.77s approved − 1.70s removals − 0.30s reshape savings + 1.50s
additions = **6.25s estimated** across 38 rows. Measured 5.36s + 6.25s
= **11.61s against the 30s budget; headroom 18.39s.** Nothing dropped;
T40 is the only remaining low-value candidate; T13 remains the success
metric's only proof — last to cut. Serial-upper-bound and link-time
qualifications carry over.

### Tripwire check on the delta

Zero on-disk deletions (all removed rows were unwritten); T17 and T36
— the two guards on existing code — survive unchanged; the
`dispatch_prompt` `push < pr < close` preservation constraint stands
verbatim. The T25–T32 fold removes tests only for behavior that is
itself removed (eligibility reads configuration, the watch reads queue
state; neither reads check results). Thinning risks re-flagged: T15's
negative half, T22's empty-herdr-log half, T7's freshness assertion,
and the skip-guard acceptance (operator host still reports 45+ passed,
0 ignored).

**Gap surfaced, not invented over:** T26's evidence half — the park
comment naming which check failed — has no successor, because no
surface for reading a `merge_group` run's failing job is locked.
DECOMPOSITION options: accept the coarser park body (dequeue reason
only, non-empty per T49), or lock a read surface and add a row.

**Verify-by-first-run additions (S8 bead):** required-check names in
the workflow must match the operator's ruleset (nothing local can
assert the ruleset half), and **the first enqueued PR leaving the
queue merged** is the single observation proving the `merge_group`
trigger, check naming, and enqueue verb agree.

### Post-delta operator decisions (2026-08-15, at the consolidated re-gate)

- **Park body is coarse for the wedge:** the evidence comment carries
  the dequeue reason (non-empty, T49), bead id, and admitted SHA;
  morning review reaches the failing job via the PR's checks tab. A
  merge_group run-reading surface waits for observed need.
- **Resolution commits are pushed by the agent lane**, like every
  worker lane; abacus's own git argv never contains `push` and the
  forbidden-flags invariant stays absolute.

---

## RECORD

Artifact produced: **ADR 0003, `docs/adr/0003-pr-validation-and-auto-merge.md`,
accepted 2026-08-15** at the operator gate. The design-document review
gate fired on creation and both role cards ran in fresh Codex panes of
a different lineage: bloat review twice (run 1 pre-amendment,
superseded; run 2 post-amendment — cut 1 withdrawn by the reviewer and
escalated into the merge-queue pivot the operator adopted; cuts 3 and
6 accepted and absorbed; cuts 2, 4, 5 reaffirmed), then spec
validation on the pivoted text (seven findings, all faithfulness
restorations of decided items, all applied). The full trail lives in
the ADR's status block.

The substage's second deliverable, the **north-star amendment**,
landed earlier in the substage at explicit operator direction and
wording (prior blob `3109a8e`, commit `da14970`): overnight merging of
pending PRs enters the success condition for opted-in repositories;
"Autonomy ends at the PR" gains the opt-in exception; non-goal 3 is
qualified. The operator held the amendment to the minimum that
licenses the decided work; a process rule was set and recorded during
this substage: NORTH-STAR.md is never edited without operator consent
on the concrete text.

Operator approval of this section: given with the ADR acceptance,
2026-08-15.

---

## DECOMPOSITION

Eight children authored 2026-08-15 under `ab-automerge-2b2`, derived
from the accepted planning state (ADR 0003 plus the gated sections
above), each carrying its test spec inline because this file is
deleted at handoff.

### Children and story traceability

| ID | Title | Stories / decisions | Final footprint |
|---|---|---|---|
| .1 | Transfer repo to org and configure the merge queue (`seat:operator`) | S1, S3, D1′ | tests/fixtures/ only |
| .2 | CI groundwork: workflow, rust-version, skip guards, BR_REAL | S7, S8, S9, D15 | .github/workflows/ci.yml, Cargo.toml, tests/br_roundtrip.rs, tests/shim.rs, bin/br-shim |
| .3 | capture_status sibling (`grp:engine-spine`) | D5′ | src/main.rs |
| .4 | abacus drain loop (`grp:engine-spine`) | success metric, S5, D3 | src/main.rs, tests/drain.rs, tests/br_roundtrip.rs |
| .5 | src/land.rs pure policy module | S1, S2, S4, S6, D5′, D11 | src/land.rs (+one mod line) |
| .6 | abacus land wiring (`grp:engine-spine`) | S1–S4, S6, D2′, D6′, D9′, D12 | src/main.rs, tests/land.rs |
| .7 | Worker prompt default branch + AGENTS.md exception | S9, D13, D14 | src/lib.rs, src/main.rs, AGENTS.md |
| .8 | Live validation: first enqueued PR merges (`seat:operator`) | S7, S8, verify-by-first-run | src/land.rs (fixture wiring), tests/fixtures/ |

### Dependencies and ready front

Requirement edges: .4 needs .2 (its br_roundtrip additions must be
authored inside the skip-guarded harness); .6 needs .3 and .5; .8
needs .1, .2, .6. Verified: `br ready` shows exactly .1, .2, .3, .5,
.7; blocked set matches intent; `br lint` clean after the epic gained
its Success Criteria section. The addendum's "all implementation
children depend on" the prerequisite was deliberately narrowed at
decomposition to the children that genuinely require the live queue
(.8) — engine code is fake-CLI-testable and honestly independent.

### Bundle groups: re-derived, retained, dropped

Retained: **A → `grp:engine-spine`** on .3/.4/.6 (all edit
src/main.rs; one lane claims all three, works them in dependency
order, one PR — the operator-approved bundling pattern). **C → .7**
as a single lane (verbatim prompt-assertion coupling). **D → .2** as
a single lane (the operator's gate-out decision made it
single-footprint). Dropped: **B** (no config surface exists to build
— D10 made opt-in invocation); **E** (the pivot collapsed conflict
resolution into .6's exception watch); **F** (D14 folded into .7,
D15 into .2; fixture-path items live in their owning beads).

### Tripwire verdict

Folded cases: additions only; the one accepted fold (T4's four-step
happy cycle) is documented in .6 with its mitigating single-leg
failure tests. Deleted tests: zero — the no-retirement constraint is
written into the epic and repeated in .2, .4, .7; the
`push < pr < close` preservation clause is a named hard constraint in
.7. Thinned assertions: the named risks live in their owning beads
(.6 carries "the negative halves ARE the assertion" on T15/T22/T7;
.2 carries the 0-ignored-on-operator-host acceptance). The review
pass proved the lens necessary: .7's original acceptance contained a
grep-to-zero clause that would have deleted the protected assertion
at src/lib.rs:373 — caught and scoped before any worker saw it
(jotted as a wording pattern for future runs).

### Freshness verdict (victor-type producer, judged from beads + HEAD only)

3 CLEAN (.4, .6, .8), 5 FIXED, 1 FLAGGED-and-corrected: the .7
self-contradiction above. Notable fixes: .1's transfer step was
already done (the operator transferred the repository mid-session;
old path redirects; local remote update and all queue configuration
remain real work — the .8 dependency stands); .2's acceptance command
was unexecutable as written (PATH excluded cargo itself — corrected);
.3's call-site count corrected to ten with the line list; .5's enum
line off by one. Every load-bearing citation (capture at
src/main.rs:311-329, the :371-378 assertion, the panic sites, the
fake-CLI harness range, retry_probe_once's error-discarding shape,
the ruleset `[]` probe, both br JSON envelopes, gh 2.87.3, the
merge-jsonl driver, PR 17) was verified live rather than assumed. All
eight children pass the Fresh Agent Test in the producer's judgment.

### Open questions

None. Every ambiguity raised in any substage was operator-decided in
its gate (tier, seven framing decisions, loop scope, br-in-CI, both
S9 de-hardcodes, the queue pivot, park-body coarseness, push
ownership, ADR acceptance) or converted into a verify-at-
implementation item recorded in the owning bead.

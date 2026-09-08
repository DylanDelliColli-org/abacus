```doc-meta
role: evidence
lifecycle: active
```

# Fantasy execution comparison — protocol and launch record

Coordinator: abacus chief of staff. Setup bead: `ab-r4m`; design discussion:
`ab-elw`. The operator approved the execution comparison, Herdr-only delegation,
independent repositories, manual abacus orchestration for A, and the lighter B
contract below. Planning is fixed and excluded from the measured execution.

## Fixed inputs

- Baseline commit: `7f3d3fc9b07b8481922dd6e0fa050f1cae4af01e`.
- Accepted plan: `docs/adr/0001-local-draft-assistant.md` in each repository.
- Work: `fantasy-p55.5` sources; `.6` draft rules; `.7` session/server; `.8`
  browser/rehearsal. Requirements and test contracts remain unchanged.
- A: `/home/ddc/dev-environment/fantasy`, remote
  `git@github-personal:DylanDelliColli/fantasy-assistant.git` for pushes.
- B: `/home/ddc/dev-environment/fantasy-experiment-clone`, remote
  `git@github-personal:DylanDelliColli/fantasy-assistant-experiment-clone.git`.
- Both repositories have independent Git metadata and independent `br` stores.
  Identical bead IDs refer to separate copies; always set the command cwd.
- Shared model configuration: GPT-6 Astra, medium reasoning. The coordinator
  offered a higher-effort alternative before setup; medium is the stated default
  unless the operator steers it before dispatch.
- Each arm may have at most two working delegates at once, including reviewers
  and delegates of managers. The leader is additional. Idle agents consume no
  active slot. Every agent uses the same model and effort for this comparison.
- No provider-native subagents. Start every worker, reviewer, and manager through
  Herdr with native multi-agent features disabled. Neither arm uses the abacus
  engine (`run`, `drain`, or `land`). Normal sandbox/approval controls remain.
- A two-hour execution checkpoint applies equally: report progress and evidence
  at that point if incomplete; do not declare success because the time elapsed.
  The coordinator can extend both equally after observing progress.

## Shared source evidence

Both arms may read the existing research downloads at these exact paths. They
are common read-only inputs, not implementation artifacts. Never overwrite them,
publish them, or depend on them for the installed application's ordinary runtime.

- `/tmp/fantasy-research-players.json` (original fetch around 16:28 UTC).
- `/tmp/fantasy-season-projections-app.json`.
- `/tmp/fantasy-season-stats-app.json`.
- `/tmp/fantasypros-half-ppr.html`.
- `/tmp/fantasy-rankings-source-summary.json`.

Live checks may access only the supplied league and its approved public provider
inputs. Automated acceptance uses fictional fixtures and actual local HTTP,
filesystem/process, and Chromium composition as specified by the plan. Human
ten-second choice speed remains unmeasured until the operator rehearses it.

## Execution and publication boundary

Both leaders start fresh in their assigned repository. Keep the initial default
branch available as the baseline. Implement on isolated worktree branches. Carry
serial dependencies forward through branch ancestry or an integration branch;
do not merge into the default branch during the comparison. Push implementation
branches and open PRs against the assigned remote. The final deliverable must
name a single integrated branch and exact commit containing all four outcomes.
Do not alter branch protection, repository configuration, or the other arm.

The operator authorizes experiment-local worker/reviewer dispatch, branch pushes,
PR creation, review comments, and the leader's engineering dispositions. This
does not authorize default-branch merges, purchases, external messages unrelated
to the experiment, or upstream Sleeper mutations. Product/plan contradictions go
to the coordinator; routine engineering decisions stay with the leader.

The accepted product test-first, unit/integration, coverage, and 30-second suite
contracts apply to both arms. A worker dispatch must include named test files,
the requirement to demonstrate a meaningful failing assertion before the change,
and then targeted/full verification. Existing planned code and tests are not
optional. Concurrent writers always use separate worktrees. After a rename or
deletion, inspect every consumer before closing the affected work.

Use `br` for work ownership/progress and `jot` for incidental discovery. Always
run them in the correct repository/worktree. Read-only review is explicitly not
bead-tracked work; reviewers do not claim/create implementation beads. Reports
are evidence, not a second tracker. Preserve the required tracker commits in the
assigned repository without contaminating an active worker's commit sequence.

Both leaders write a short launch acknowledgement and later milestone reports
under their assigned report directory, with current branch/head, worker names,
tests actually run, review dispositions, and any coordinator intervention needed.
At completion write `final.md` and `ready.json` there. `ready.json` includes arm,
repository, branch, full commit, PR URLs, actual checks/results, limitations,
start/end timestamps, and the independent internal review artifact(s). It is a
completion receipt; work state remains in beads. The common final evaluation is
performed separately after each leader declares ready.

## Arm A — manual abacus

Launch brief: [Leader A](2026-09-08-fantasy-leader-a.md).
Leader name: `fx-a-lead`; reports: `/tmp/fantasy-experiment-a`.

The leader manually performs dispatch, tracking, review launches, engineering
adjudication, and rework through Herdr. This explicitly extends the ordinary
abacus skill's manual exception beyond evaluator launch, as directed by the
operator. It must not pause because the skill reserves a transition to the
engine, nor build replacement engine machinery.

Preserve the standing worker and three-seat review methodology: correctness
gates; simplicity and test quality advise; every finding gets a disposition.
Use a fresh reviewer context per seat and cycle on the same exact revision.
Respect the two-delegate cap by scheduling the three seats in batches. The
matched Astra-medium setting supersedes ADR 0007's standing Sol-medium setting
for this experiment only. No protocol repair work is part of the product build.

## Arm B — agent-directed execution

Launch brief: [Leader B](2026-09-08-fantasy-leader-b.md).
Leader name: `fx-b-lead`; reports: `/tmp/fantasy-experiment-b`.

The leader owns grouping, delegation, scheduling, routine recovery, and extra
verification while honoring the fixed plan and real dependency constraints. It
may implement directly. At least one fresh independent reviewer must inspect
the integrated result before the leader declares ready. Earlier reviews and
specialists are the leader's choice within the shared resource limits.

This experiment contract replaces inherited abacus execution choreography for B
and its delegates. Do not apply the abacus execution/planning skill as mandatory
procedure, the three standing review seats, exact adjudication grammar, or fixed
review-cycle rules. Preserve the shared authorization, isolation, durable state,
product requirements, and test contracts. No global instruction file is edited.

The approved review clause is:

> Reviews that accept or refute the changes can both be valid results. The
> conclusion should follow from the evidence.

## Observation and evaluation

Record actual prompt-submission/engagement times, agent names/configurations,
operator engineering interventions, clarification shared with both arms, elapsed
time, available token/cost evidence, defects, and rework. Do not fabricate cost
estimates as measurements. Setup and prior planning are reported separately.

Evaluate the same frozen requirements and real application behaviors on each
integrated head. Keep code and review findings separate until both declare ready.
The reviewer minimum inside B is part of B's execution cost; the common final
comparison is a separate measurement. A single project is a pilot, not proof of
general superiority. Preserve incomplete/failed outcomes honestly.

Launch workspace IDs, exact submitted prompts, observed engagement, and progress
are recorded on `ab-r4m` and the follow-on monitoring bead. Neither arm is
accepted or released merely because its worker beads close.

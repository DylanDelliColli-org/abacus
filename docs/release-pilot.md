```doc-meta
role: contract
lifecycle: active
```

# Release-led execution pilot

One chief of staff owns delivery. Agents choose the engineering approach;
independent evidence determines whether an integrated candidate is acceptable.
This packet is an opt-in pilot, not the default abacus workflow. Its presence
does not activate it, change installed skills, or authorize a deployment.

## The packet

- [Chief of staff](release-chief-of-staff.md): execution discretion and delivery ownership.
- [Release reviewer](release-review.md): independent product acceptance.

Use the existing `br` tracker and Herdr agents. No new engine, installer,
review-status parser, or CI service is required. Keep the measured lessons in
[CONSTRAINTS.md](../CONSTRAINTS.md) and the purpose in
[NORTH-STAR.md](../NORTH-STAR.md); this pilot does not amend either.

## Activate one bounded pilot

Choose a real repository and an approved product task first. Keep the accepted
product requirements and planning decisions; this pilot changes execution, not
the planning process or product scope. Do not enroll other active tasks by inference.

The chief prepares an isolated worktree and a read-only checkout of this packet
at an exact commit. Record the packet path and commit, target repository,
task/epic bead, product acceptance source, integration branch, Herdr workspace,
and merge/deployment authority on the target task. Keep runtime data, credentials
and browser caches isolated; a worktree does not isolate shared services or trackers.

Have the operator approve that concrete scope with this instruction, completed
with actual values before sending it to the chief and its delegates:

> For repository [absolute path], task [bead], integration branch [branch],
> use the release-led pilot packet at [absolute path], commit [full SHA].
> Read docs/release-pilot.md and docs/release-chief-of-staff.md there.
> Product acceptance is [source].
> Use Herdr workspace [ID] and the existing resource/model settings.
> Within this task, these instructions supersede inherited abacus execution
> requirements for fixed per-PR reviewer panels, adjudication grammar, and
> engine-controlled dispatch. They do not override higher-priority instructions,
> product requirements, planning approvals, test obligations, tracker/capture
> policy, permissions, isolation, or merge/deployment restrictions.
> Approved merge/deployment authority is [existing authority or none].
> No other repository or task is opted in.

This is a task-scoped user instruction, not authority the agent grants itself.
The chief verifies the effective instructions before dispatch. If a remaining
instruction conflict would change authority, stop and resolve that conflict;
do not disable hooks, edit global files, or evade branch protection. Supply the
pinned packet to resumed agents too. Do not silently follow a moving branch tip.

## Try it without installing it globally

Leave the normal abacus checkout and installed skills alone. On the current
host, the planning skill and `br` shim point into that checkout; changing it can
affect other repositories. The installed execution skill may be a separate copy.
Use explicit paths to this packet rather than replacing those installations.

The chief manages build, independent evaluation, and repairs. The operator
provides product direction and any genuinely new authority, not reviewer traffic.
Work toward a usable release candidate; do not postpone all integration until
the end of a large project. Neither a pushed branch nor technical acceptance
grants permission to merge or deploy.

## Evidence for the pilot decision

Use the target bead for work state and links to durable evidence. Record the
actual work start, original first candidate ready time, evaluation and repair
intervals, accepted revision/time if reached, and operator interventions with
their reasons. Keep holds and waiting separate; elapsed time is not active model
time. Report tokens/cost only when measured. Preserve first failures after repair.

Compare time to accepted output, unresolved/escaped defects, and the burden on
the operator. A passing smoke test alone is not acceptance. A prepared packet
is not a successful real-work pilot. Do not automate away an inconvenience
until actual use establishes what needs fixing.

## Withdraw or finish

Stop new pilot dispatches, let active work reach a safe checkpoint, and record
its commits, outstanding findings and ownership on the target bead. Preserve
worktrees and evidence. Remove only this task's activation on operator direction;
do not kill agents, discard work, revert production, or rewrite global settings.
Existing work does not become accepted merely because the pilot ends. Applying
the workflow more broadly is a separate operator decision.

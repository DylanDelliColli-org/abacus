## Simplicity review

This is the manual-mode brief template. Before launch, replace every target
placeholder:

- Repository: `<repo>`
- PR: `<number>`
- Reviewed head: `<full 40-character sha>`
- Bead and acceptance claims: `<id and claims>`
- Authority and changed paths: `<paths>`
- Changed or behaviour-implicated tests: `<paths, or none>`

Work as a fresh simplicity reviewer. Your question is: **is this the right shape for what the bead asked?**
Produce proposals, never blockers. There is no severity floor and no
executed-failure requirement; speculation is welcome. Your review is advisory
and does not gate the merge.

### Posture and authorized output

The review is read-only. Inspect the PR, its authority, and relevant tests,
but do not edit files, commit, push, change branches, or mutate repository or
PR state. Your exactly one permitted PR write is the authorized deliverable:
post exactly one PR comment with
`gh pr comment <PR> --body-file <REVIEW_FILE>`.

The comment's first body line must be exactly `## Simplicity review`. Put no
preface above it. Add no cycle number and no verdict line. If the permitted
post is denied, return the complete body to the orchestrator for verbatim
relay; do not substitute another mutation or post a second comment.

This read-only review is explicitly not bead-tracked work. For a pre-existing
finding outside the PR, capture it with
`jot "<observation>" --file <path> --symptom "<symptom>" --repro "<breadcrumb>"`.
Capture only; do not create, claim, or update a bead.

### Objective: concepts, not lines

The reviewer targets unnecessary code and unnecessary concepts — names a reader must hold: types, abstraction layers, configuration knobs, control-flow branches, files.
Line count is not the objective and is never cited as a reason.
A change that adds lines while removing a concept is a valid simplification, as is one that adds lines to eliminate a nested ternary or a dense one-liner.
If a proposed simplification would require changing an existing test, it is a behaviour change, not a simplification — report it as such or drop it.
A proposal that increases both lines and concepts must state explicitly why; an unjustified one is a defect in the review.

### Production-shape probes

Ask whether the implementation introduces names, branches, files, layers, or
configuration that the requested guarantees do not need. Prefer the existing
repository's idioms when they express the same guarantee with fewer concepts.
Do not reward density or obscurity.

For a PR that is itself a reduction, ask both: did the reduction overshoot,
and what adjacent bloat remains? Restoration is a valid finding.

Exclude generated code, vendored dependencies, language idioms, sub-5-line
snippets, and code the bead's acceptance criteria require.

### Narrow test review

Inspect changed or behaviour-implicated tests only; never produce a suite
inventory. The unit of analysis is **behaviours and assertions**:

- duplicate behavioural proof;
- assertions no longer pinning the named contract; and
- coverage lost through folding, deletion, or thinning.

Keep any proposal within the existing behavioural contract. Apply the
existing-test rule in the concepts-not-lines objective before proposing a
test simplification.

### Proposal contract

Report only proposals that clear a significance threshold and rank them
most-significant-first. There is no numeric cap; reporting nothing is a valid and expected outcome.

For every proposal state:

1. what is removed;
2. which guarantee survives and how it is checked; and
3. the rough cost of the change.

End the comment with a `### Considered and rejected` section stating what you
considered and rejected, and why. Include the section even when there are no
proposals; it is the calibration signal that cutting was not the objective.

### Framing

Frame checks as correctness invariants and use “exercise” for concrete probes.
For authentication or credential subject matter, describe the mechanics with
no domain vocabulary. Avoid adversarial action verbs throughout the brief,
probes, and final comment.

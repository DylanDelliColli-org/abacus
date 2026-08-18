# ADR 0005 spec-validation findings

Verdict: five findings. The record is not implementable as written until the
state-cycle blocker and the contradictions below are reconciled. This was a
refinement-only review against the approved FRAMING, RESEARCH, ARCHITECTURE,
and TEST-STRATEGY record plus `br show ab-phr`, `br show ab-co5`, and `br show
ab-blocked-lane-outcome-6bs`. The Status block's bloat dispositions were
treated as settled; scope was not reconsidered.

## Findings

### 1. Blocker — the stateless probes cannot distinguish a consumed rework/review cycle from an unconsumed one

`docs/adr/0005-lane-lifecycle-v2.md`, D1–D2, lines 68–85 says `LaneState` is
“re-derived per cycle” from substrate probes, that the engine “persists
nothing,” and that transitions include both launching reviewers and
redispatching rework. D4, lines 101–117 then narrows the durable textual input:

> “The engine machine-parses exactly two textual signals ever: the worker's
> `BLOCKED` comment token and adjudication comments. Reviewer verdict bodies
> are never machine-parsed.”

The same section says `pending` is posted once and only an accepting
adjudication flips it. The approved ARCHITECTURE makes the cross-cycle problem
explicit: `PLANNING-ab-lifecycle-v2-go4.md`, A3, lines 461–468 keeps that same
status pending through rework cycles, while A4, lines 478–487 redispatches from
the rework adjudication and preserves the same agent and branch.

Concrete failure: after cycle *k*'s adjudication requests rework, D5 prompts the
worker. When the worker finishes that rework and closes the bead again, the
latest machine-parsed adjudication still requests rework and the PR status is
still the same pending status. A fresh sweep therefore reconstructs the same
`ReworkRequested` evidence and can redispatch cycle *k* again. If closed-bead
precedence instead moves the lane to `AwaitingReview`, a crash/restart cannot
tell whether that cycle's ephemeral reviewer already posted its deliberately
unparsed verdict and can launch a duplicate review. Both paths violate D2's
crash reconstruction and D3/D5's one-fresh-review-then-warm-rework sequence.

The existing state derivation must identify which already-approved durable
observation distinguishes an unconsumed adjudication and an unperformed review
cycle from consumed/performed ones. This is required to make D1–D5 executable;
it does not add a new workflow.

### 2. High — D6 assigns both exit 0 and exit 3 to the ordinary closed-bead/open-PR settle

`docs/adr/0005-lane-lifecycle-v2.md`, D1, lines 63–73 says:

> “a closed bead is `Completed` regardless of comments”

The approved definition in `PLANNING-ab-lifecycle-v2-go4.md`, A1, lines
407–416 simultaneously defines `AwaitingReview` as “bead closed + PR open + no
accepting adjudication.” D6 of the ADR, lines 131–137 then says `abacus run`
keeps `0 = Completed` but returns 3 for `AwaitingReview`.

Concrete failure: a normal close-last worker settle with its PR still open is
both `BeadOutcome::Completed` and `LaneState::AwaitingReview`, so D6 requires
both exit codes. If lane state wins, the claimed preserved `0 = Completed`
behavior no longer covers an ordinary successful lane; if bead outcome wins,
the expressly ruled `AwaitingReview` exit 3 is unreachable for that lane. The
TEST-STRATEGY exit-code ruling (`PLANNING-ab-lifecycle-v2-go4.md`, lines
663–670) cannot be implemented deterministically until D6 states which layer
owns the result and how this overlap resolves.

### 3. High — D4 drops the approved per-finding adjudication dispositions that D5 needs

`docs/adr/0005-lane-lifecycle-v2.md`, D4, lines 101–107 reduces the second PR
comment to an adjudication heading “with an accepted or rework verdict.” The
accepted input is more specific. `br show ab-phr`, Comments, 2026-08-18 14:09
UTC requires the follow-up to record:

> “per finding: accepted, rejected, or rerouted, with destinations
> (rework-spec bead comment, out-of-scope bead IDs, fix commit SHAs).”

RESEARCH preserves that shape at
`PLANNING-ab-lifecycle-v2-go4.md`, lines 314–316, and approved A4, lines
478–481 says the rework spec is generated from the adjudication's accepted
findings and commit expectations. D5 of the ADR, lines 119–122 still requires
generation from that comment, but D4's stated grammar permits only an overall
“rework” verdict with no finding-level disposition or destination.

Concrete failure: with mixed accepted, rejected, and rerouted findings, an
implementation conforming to D4 cannot tell which findings enter the warm
worker's rework spec or preserve the required routing/audit trail. D4 must
retain the already-approved per-finding dispositions and destinations; this is
preservation of the two-comment contract, not a new requirement.

### 4. Medium — the Herdr summary claims more topology coverage than RESEARCH measured

`docs/adr/0005-lane-lifecycle-v2.md`, Context, lines 52–56 says:

> “`agent wait --until idle` fails everywhere; `agent prompt --wait` and
> `agent wait --until done` work in all tested topologies.”

The RESEARCH matrix at `PLANNING-ab-lifecycle-v2-go4.md`, lines 224–247 is
narrower: settled-agent `--until idle` timed out in one split-pane and one
dedicated-root probe; pre-first-turn `--until idle` succeeded; `prompt --wait`
worked in both topologies; and `--until done` was exercised only in the
dedicated root pane. RESEARCH also scopes the result to codex-kind agents on
the current Herdr build.

Concrete failure: the ADR licenses `agent wait --until done` in a split-pane
topology as experiment-backed even though that cell was never measured, and
its unqualified “fails everywhere” contradicts the measured pre-first-turn
success. The summary should preserve the measured boundaries: settled codex
agents for the idle failure, both topologies for `prompt --wait`, and only the
dedicated-root probe for `--until done`.

### 5. Medium — the GitHub summary turns a plan-gate result into an unsupported no-auth claim

`docs/adr/0005-lane-lifecycle-v2.md`, Context, lines 57–59 concludes that
required-check enforcement availability is “a per-repo fact, not an auth
question.” RESEARCH at `PLANNING-ab-lifecycle-v2-go4.md`, lines 262–279 instead
records two separate facts: DylanDelliColli had push but not admin,
DDC-Heartwood had admin, and the private user-owned free-plan repository
returned the same plan-upgrade 403 even to the admin account. It expressly
describes this as separating “account-level admin” from “plan-level
availability.”

Concrete failure: the ADR's wording treats authentication/authorization as
ruled out generally, although the experiment established only that switching
between these two accounts could not overcome this target repository's plan
gate. Onboarding could consequently diagnose every required-check
configuration failure as plan-based without evidence. The record should say
that plan availability is an independent per-repo gate and that it was the
blocking gate on the measured target; RESEARCH did not establish that auth is
irrelevant on plan-eligible repositories.

## Not checked

- I did not redo the bloat/scope review, reconsider any of its four cuts, or
  evaluate the possible future evolution recorded only in the Status block.
- I did not inspect or modify implementation code, derive decomposition beads,
  run tests/builds, or test whether the proposed state machine happens to be
  recoverable through behavior not stated in the ADR.
- I did not run live Herdr experiments, GitHub API writes, branch-protection or
  ruleset changes, or independent `br show --json` shape probes. Substrate
  validation was limited to checking the ADR's claims against the approved
  RESEARCH measurements and the three named read-only bead records.
- I did not independently inspect market-brief PRs 25/26, the referenced
  bb-skills launching guide, or the production timing observations.
- I did not validate ADRs 0001–0003 beyond the relationships attributed to
  them here. I checked only the quoted `NORTH-STAR.md` success-condition anchor,
  not the rest of those decisions' implementation history.

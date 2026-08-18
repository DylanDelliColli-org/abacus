```doc-meta
role: working
lifecycle: inflight
```

# PLANNING — ab-lifecycle-v2-go4: Lane lifecycle v2

**Tier: FULL**, operator-confirmed 2026-08-18. Rationale: the cluster
introduces new contracts (an engine lane-outcome model, a required
`adversarial-review` GitHub status check, the two-comment PR verdict
convention, a templated refutation brief); it contains a real architecture
decision (the engine is currently strictly serial and stateless — warm lanes
awaiting review while a drain continues means concurrent live lanes tracked
across invocations); and it carries two live unknowns (the `herdr agent wait`
split-pane false-fail boundary; the check-flip auth identity, where today's
default-branch flip already required the DDC-Heartwood account). Input specs:
`ab-phr`, `ab-co5`, `ab-blocked-lane-outcome-6bs` — each operator-directed,
each carrying production precedent from market-brief-package 2026-08-17/18
(PRs 23–26).

## FRAMING

Drafted by the orchestrator from the operator's recorded rulings in the three
input beads; presented live for the FRAMING gate 2026-08-18.

### User stories

- **S1 — automatic adversarial review.** After a lane settles with a PR, an
  adversarial reviewer launches automatically as a fresh codex context in its
  own dedicated herdr workspace, handed a refutation brief generated from the
  bead spec, so every lane PR receives refutation-grade review without manual
  choreography. (From `ab-phr`; pane choreography per bb-skills
  `agents/LAUNCHING.md`.)
- **S2 — durable two-comment verdicts.** The reviewer posts its full
  unadjudicated verdict as one PR comment (its only permitted write); after
  the operator rules, the orchestrating side posts the adjudication
  follow-up. Raw verdicts are durable evidence; nothing blocks on them until
  adjudicated. (From the `ab-phr` amendment; precedent backfilled on
  market-brief PRs 25/26.)
- **S3 — structural enforcement.** Lane PRs carry a required
  `adversarial-review` status check that stays pending until the adjudication
  comment flips it, so an unreviewed PR cannot merge even by habit. (From
  `ab-phr`.)
- **S4 — warm rework.** When the operator's ruling requires rework, the
  rework spec is prompted into the SAME warm worker on the same lane branch;
  the lane reaps on merge or explicit abandon, not on settle. Measured basis:
  fresh rework lanes cost 8–15 min vs ~5 min warm (one bead paid the
  orientation tax three times). (From `ab-co5`.)
- **S5 — drain resilience.** An unattended drain continues past lanes that
  settle blocked or awaiting-review instead of aborting the night, and a
  reopened-for-rework bead redispatches into its warm agent rather than
  minting a fresh lane. (From `ab-blocked-lane-outcome-6bs` + `ab-co5`;
  production basis: three same-branch rework cycles clean under single
  `abacus run` invocations, so the gap is drain-specific.)
- **S6 — legible morning report.** Every lane settle is reported by class —
  completed / blocked / awaiting-review / reopened-rework / stalled — so the
  operator reads one report instead of opening panes. (From
  `ab-blocked-lane-outcome-6bs`.)

### Non-goals

- **N1 — no CI-authoritative review.** Operator considered and rejected
  2026-08-18: CI loses the jot funnel and the operator interrogation
  surface. The optional non-blocking CI commenter mentioned in `ab-phr` is
  out of this epic entirely; any API-key-in-CI variant needs a fresh operator
  ruling.
- **N2 — no API-key reviewer auth.** Codex subscription OAuth only (standing
  ruling 2026-07-20).
- **N3 — no warm reviewers.** Reviewers are ephemeral, one fresh context per
  review cycle. The author-warm / reviewer-ephemeral asymmetry is deliberate
  design, not an optimization target.
- **N4 — no automated adjudication.** The operator rules on findings after
  verifying them against real code; machinery only records verdicts, flips
  the check after the ruling, and routes rework.
- **N5 — no BLOCKED-lane resume machinery.** A blocked lane parks with its
  durable comment; un-blocking remains manual. (Warm rework under S4 applies
  to review-driven rework, not to blocked lanes.)
- **N6 — no general parallel-dispatch scaling and no bundling.** Concurrency
  enters only as far as the cluster requires: settled lanes may await
  adjudication while the drain proceeds. Cluster/bundle selection stays with
  `ab-bundling-plan-4mu` (a separate parked planning trigger for engine-side
  lane bundling).

### Epic success metric

An unattended `abacus drain` over at least three ready beads in an onboarded
repository ends with: every settled lane classified (zero drain aborts from
blocked or awaiting-review states), every lane PR carrying a reviewer verdict
comment and a pending required check, and any rework cycle dispatched into a
warm agent — with zero manual pane interventions between first dispatch and
the morning report.

### Narrowest valuable wedge

The outcome model plus drain continuation (`ab-blocked-lane-outcome-6bs`
scope): the drain classifies blocked / awaiting-review / rework settles and
continues instead of aborting, with per-class reporting. This ships value
alone — it converts the observed drain-killing failure into a parked lane —
and every later story builds on its classification. Review-gate automation
(S1–S3) and warm-lane keepalive (S4) layer on top.

### Prerequisites

None. The two live unknowns (split-pane `agent wait` boundary; check-flip
auth identity) are RESEARCH items inside this run, not blocking beads.
`ab-24o` (sandbox worktree-index bead, filed by another session) is judged
unrelated pending RESEARCH confirmation.

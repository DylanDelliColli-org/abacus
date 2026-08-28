```doc-meta
role: working
lifecycle: active
```

# Operating the lifecycle — what the engine does and where you sit in it

The binding decisions live in `docs/adr/0005-lane-lifecycle-v2.md`. This
document does not restate them; it answers the three questions an operator
gets wrong when reading the ADRs cold, and cites the decision that governs
each.

It exists because of a measured failure. On 2026-08-20 an orchestrator with
no prior engine knowledge read `NORTH-STAR.md` and `docs/adr/`, ran
`abacus run` against a multi-bead chain, and concluded from what it saw —
exit 0, the bead closed, the PR still draft, the lane in `AwaitingReview` —
that the engine had an acceptance-gate inversion and a one-shot review
cycle. It filed four notes against designed behavior and advised re-running
the command that had already done its job. Every inference was reasonable
from the documents available. The documents were the defect.

## 1. `run` dispatches one bead. `drain` is the loop.

`abacus run` dispatches **at most one** ready bead and settles. Exit 0 means
the nominal settle was reached — bead closed, PR up, reviewer launched
(`AwaitingReview`) or `Merged`. Exit 3 means a parked settle (`Blocked` or
`Stalled`); exit 1 means engine failure (ADR 0005 D6).

`abacus drain` is the loop. Each iteration re-derives every live lane, acts
on its transitions — launch reviewer, flip status, redispatch rework, reap
merged, park blocked — and only then dispatches at most one new ready bead
(ADR 0005 D2). Review reconciliation happens **only** in the drain.

Under ADR 0006, review launching is mode-dependent: engine mode launches automatically; in operator-declared manual mode, the orchestrator launches per the `abacus-execute` procedure.

So a `run` that exits 0 with one bead closed and a lane awaiting review has
done exactly what it promises. It is not a truncated drain. If you want the
ready front continuously cleared and reviews reconciled, the command is
`abacus drain`.

## 2. A closed bead means the author is finished. It does not mean accepted.

This is the inversion that reads as a bug and is not one.

The worker closes its bead when its own contract is satisfied — tests
written and passing, PR opened. That close is an **author-done** signal. It
carries no claim that the work was accepted, and the engine does not treat
it as one: a closed bead whose lane is `AwaitingReview` resolves in
lane-state's favor, not the bead's (ADR 0005 D6).

Acceptance lives downstream, in two places:

- The **adjudication comment** you post by hand on the PR, in the byte-exact
  grammar of ADR 0005 D4. An accepting adjudication is the only thing that
  flips the `adversarial-review` commit status from `pending` to `success`.
- The **required check at merge**, on repositories where that status has
  been made required. Making it required is an onboarding act, never
  something the engine configures (ADR 0005 D4).

An unadjudicated `REFUTED` verdict blocks nothing on its own. The verdict is
a reviewer's report; the ruling is yours.

## 3. The adjudication gate is human-hand-posted, and invisible if you do not know it exists.

The engine launches reviewers and parses adjudications. It **never writes
one.** After a reviewer posts its verdict, the lane sits in
`AwaitingReview` until a human posts the adjudication comment on the PR.

The drain does not block on this and never aborts because of it — it
re-derives, finds no transition available, and moves on. Nothing in the
output says "waiting for you." So an operator who does not know the
adjudication grammar exists will leave every lane waiting forever on a gate
they cannot see, and the drain will keep exiting 0 while nothing advances.

The grammar, the per-finding disposition vocabulary, and the
adjudicated-head anchor are specified in ADR 0005 D4 and operationalised in
`.claude/skills/abacus-execute/SKILL.md`. If you are adjudicating, read the
skill; if you are extending the parser, read D4.

## Known documentation conflict

`NORTH-STAR.md` currently states: *"Acceptance happens inside the agent team
— a reviewer accepts, the bead closes."*

Section 2 above describes what the engine actually does, and the two do not
agree: the bead closes before review, and acceptance is the operator's
hand-posted adjudication plus the merge-boundary check. ADR 0005 D3–D4 are
the binding decisions and this document follows them.

Reconciling the north-star sentence is an amendment, which happens only
through `/north-star` revise mode — an explicit operator act, never a
consequence of an inconvenient check. This note records the conflict; it
does not resolve it.

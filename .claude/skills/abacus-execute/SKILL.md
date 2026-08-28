---
name: abacus-execute
description: >
  Operate the orchestrator seat of an abacus-governed repository: run the
  drain, adjudicate adversarial reviews in the byte-exact grammar, relay
  sandbox-denied verdicts, gate closes, and end sessions with verified
  durable state. Use when orchestrating abacus execution in any repo, when
  asked to "/abacus-execute", "drain the pool", "shepherd this PR through
  review", or when an abacus lane is awaiting your adjudication.
---

# /abacus-execute — operate the orchestrator seat

The engine is the machinery; you are the human gate. This skill is the seat
contract. Every rule below exists because an orchestrator without it got the
flow wrong in the field (contest 2026-08-20, market-brief 2026-08-21).

## 1. Role split

Three seats, with two operator-selected modes for evaluator launch. Do not do
another seat's work.

- **Engine** (`abacus run` / `abacus drain` / `abacus land`): dispatches
  workers into lanes, classifies every settle, recovers paste races, launches
  and reaps evaluators, parses adjudications, posts the `adversarial-review`
  commit status (PR head and merge-group commits), and reaps merged lanes. In
  **engine mode**, the engine launches, tracks, and reaps the evaluators.
  **Engine mode only:** Never hand-perform these; if the engine misbehaves,
  capture evidence and escalate — do not build a manual workaround loop.
- **Orchestrator** (you): read verdicts, adjudicate them, reopen beads for
  rework, relay sandbox-denied verdict posts, keep the tracker committed and
  pushed, escalate anything identity-bearing or configuration-shaped. In
  manual mode, also launch evaluators and collect their reports by the written
  procedure below; this exception does not authorize hand-performing any
  other engine transition.
- **Operator** (the human): repository configuration (rulesets, branch
  protection, merge queues, CI workflows — ADR 0003 forbids the engine and
  you from mutating these), merge grants, auto-merge opt-in via `land`,
  jot-queue curation, and anything recorded under their identity that they
  have not explicitly delegated.

**Manual mode** is an operator-declared, permanent first-class fallback. The
operator declares it per repository or per session; the orchestrator never
infers it. If the active mode is ambiguous, ask the operator before launching
evaluators. Manual mode authorizes the orchestrator procedure below; it does
not become an improvised replacement for the rest of the engine.

Both modes obey **mode equivalence at the comment stream**: they produce
byte-compatible PR artifacts with the same headings, verdict grammar,
adjudication grammar, and cycle semantics. A mode switch in the middle of a
review arc therefore strands nothing; the same PR, ledger, and cycle sequence
continue.

Autonomy ends at the PR unless the repository is opted into overnight
merging. A merge grant is session-scoped and explicit; never carry one
forward from a previous session or another repo. `--admin` merges are
forbidden always.

## 2. The operating loop

```
abacus drain          # dispatch ready beads, sweep lanes, launch/reap reviewers
# read the report, then for each lane holding a NEW reviewer verdict:
#   adjudicate (section 4)
#   REFUTED  -> reopen the bead (br update <id> --status in_progress); the
#               next drain routes warm rework to the preserved author lane
#   NOT REFUTED -> post the adjudication; the next drain flips the
#               adversarial-review status; then the merge path (section 7)
abacus drain          # advance the cycle you just adjudicated
```

`abacus run` dispatches exactly one bead and exits — exit 0 with a lane
parked awaiting-review is its nominal result, not a completed drain. Use
`drain` for the loop. Exit codes: 0 nominal (including AwaitingReview),
3 Blocked/Stalled, 1 engine failure.

### Manual-mode operating loop

For every lane in `AwaitingReview`, run both evaluators on the same reviewed
head and in parallel. Give correctness the self-contained manual brief below
and give simplicity [`simplicity-brief.md`](simplicity-brief.md), with all
target placeholders resolved. Both evaluators run on every cycle; neither has
a launch predicate.

Collect both reports before ruling. Adjudicate them together by section 4:
the correctness verdict controls the gate, while every simplicity proposal
gets a disposition inside the same correctness adjudication comment. On a
`REFUTED` ruling, reopen the bead with
`br update <id> --status in_progress` exactly as the engine loop does, then
let the normal warm-rework route continue. On `NOT REFUTED`, post the
adjudication and advance the configured merge path. Repeat until the PR
converges.

## Manual evaluator launch procedure

Create one review worktree per evaluator. Substitute collision-free `NN` and
`K` identifiers for each workspace, and run from a shell that can reach the
target repository:

```sh
herdr worktree create --cwd <repo> --branch review/prNN-cK \
  --base origin/lane/<bead> --label review-prNN-cK
```

Read the pane identifier from `.result.root_pane.pane_id`. After creating the
workspaces, wait **12-14 seconds** for pane readiness before starting either
agent. The measured `agent_pane_busy` window is **10-20 seconds**, and can be
longer under load.

Start each agent in its pane. Herdr forwards every argument after `--` to the
provider CLI:

```sh
herdr agent start <name> --kind codex --pane <id>
herdr agent start <name> --kind codex --pane <id> -- --model <m> -c model_reasoning_effort=<e>
```

Use optional model and effort overrides only when the review calls for them.
Prefer the equivalent `-m <m>` spelling to `--model <m>`, and use only model
strings that are safe to encode as shell arguments.

Prompt each evaluator with its completed brief and require engagement proof:

```sh
herdr agent prompt <name> "<brief>" --wait --until working
```

`--until working` returns when the prompt is engaged, not when the turn ends;
that proof closes the paste-race window and makes parallel launch reliable.
Create both worktrees, start both agents, prompt both agents, and only then
wait. In the background, run one independent subscription per evaluator:

```sh
herdr agent wait <name> --until idle --until done --until blocked
```

If a subscription outlives the expected **10-20 minute** review, switch to a
bounded set of fresh status probes. A long-lived wait has been observed
**25 minutes past idle**. Trust `agent_status` and the tracker, never a pane
read; pane contents are diagnostics, not liveness.

## Evaluator heading registry

- Correctness owns `## Adversarial review — cycle <n>`.
- Simplicity owns `## Simplicity review`, with no cycle number and no verdict
  line.

No evaluator other than correctness may post a heading beginning
`## Adversarial review — cycle `. This is a hard safety rule: the engine
prefix-matches that heading and ignores trailing text, so a collision creates
a phantom cycle that can kill a live reviewer, suppress relaunch, and clear an
unreviewed merge if the phantom is adjudicated.

## 3. State semantics — the table that prevents misreads

| Observation | Meaning | NOT |
|---|---|---|
| Bead `closed` | The author finished and pushed; dispatch contract is push, open PR, close bead | Acceptance. The gate is still open |
| Lane `AwaitingReview` | A verdict or adjudication is owed | A stall |
| Verdict posted after bead closed | Nominal ordering — reviewers run detached and post minutes later | An inversion |
| Verdict exists, nothing advances | The engine is waiting on YOUR adjudication — it will wait forever | An engine bug |
| Bead `in_progress` after REFUTED | Rework routing state you created by reopening | A worker error |
| Pane shows the prompt unsent at 0% context | Possibly a redraw artifact — reviews have been in flight behind exactly this pane state | Proof the agent never started |

Acceptance = an authorized, well-formed adjudication accepting the verdict
at the current head, then the merge. Nothing else is acceptance.

## 4. Adjudication grammar — byte-exact, machine-parsed

Post as a PR comment. The engine parses it; deviation makes it invisible or
inert. Only the allowlisted operator login with OWNER/MEMBER association
counts, and the cycle must match a parsed reviewer-verdict cycle on the PR.

```markdown
## Adjudication — cycle <k>

Verdict accepted: NOT REFUTED.

Finding 1 (<severity> — <summary>): ACCEPTED <reasoning>.

Adjudicated head: <full 40-character sha>
```

Rules that have burned sessions:

- `Verdict accepted: NOT REFUTED.` or `Verdict accepted: REFUTED.` — the
  period immediately follows; prose only after it.
- `Adjudicated head:` is the full sha the accepted verdict actually
  reviewed. If the head moved since, do not adjudicate-accept at the new
  head — re-review first.
- One `Finding N (...): ACCEPTED|REJECTED ...` line per verdict finding.
- A REFUTED adjudication alone does not dispatch rework: reopen the bead.

The fenced grammar stays unchanged. Treat the adjudication as one transaction,
not a staging point. In the same comment, after the required machine-parsed
lines, add a labelled `Review transaction:` paragraph naming the surfaces
examined, material exclusions, and the completeness judgement; the required
`Adjudicated head:` line supplies the reviewed head.

Add a labelled `Simplicity review:` paragraph naming the disposition of every
simplicity proposal. It is part of the correctness adjudication comment and
does not gate the merge; it never gets its own `## Adjudication` heading or a
cycle number. Every correctness concern and simplicity proposal receives its
durable disposition in this operation: fold it into current rework, name the
new or existing bead ID in the comment, or explicitly reject it with reasons.
Deferred filing is forbidden.

## Convergence controls

Maintain a per-PR class ledger derived from that PR's verdict and adjudication
comments. Never create a repository-wide ledger file. Record finding classes,
dispositions, regression probes, and enforcement seams, and generate each
cycle's evaluator briefs from this durable history.

A finding re-blocks only when its class is new or a recorded regression is
demonstrably live. At the second instance of a class, move the guard to the
narrowest seam that covers the whole class and test a sibling. At the third instance,
stop rework, put the design question to the operator, and retire the warm
author's accumulated context: start a fresh agent in the same worktree, branch,
and PR. Always regenerate rework prompts from durable state; never use "address cycle N" as the prompt.

## 5. Verdict relay — reviewer sandbox denials

Engine-launched reviewers intermittently have their `gh pr comment` denied
by their own sandbox. The reviewer saves the verdict to a file and asks for
authorization. Protocol:

1. Read the verdict file; post it verbatim:
   `gh pr comment <n> --body-file <file>`.
2. Attribution goes AFTER the verdict body — a `---` rule, then one italic
   line stating you relayed it.
3. Never put any line above the `## Adversarial review — cycle <n>`
   heading. The parser requires the heading as the first body line; a
   prefaced relay is invisible and the drain relaunches the same cycle.

Apply the same no-preface relay rule to simplicity. Its first body line must
be exactly `## Simplicity review`; post no cycle number and no verdict line.
Relay the saved body verbatim before adding attribution below it.

## Manual reviewer brief — self-contained correctness contract

In operator-declared manual mode, give every correctness reviewer a brief
that produces the same comment-stream artifact as engine mode. Name the
repository, exact PR and reviewed head, bead and authority path, required
`## Adversarial review — cycle <n>` heading, verdict grammar, and verdict
file. State that `gh pr comment <n> --body-file <file>` posts the reviewer's
authorized deliverable and is exactly one permitted write; everything else
is read-only.

Supply a gate-scope block rather than making the reviewer rediscover it:

- the focused suite selected for the changed paths;
- the import-provenance check and expected reviewed checkout;
- known environment issues, with `none supplied` written explicitly when
  the list is empty; and
- room for reviewer-authored targeted probes. A broader gate is permitted
  only for a specific finding, with the reason recorded under Probes. Do
  not ask for a default full-suite rerun.

Carry these correctness clauses verbatim from `REFUTATION_BRIEF_TEMPLATE`;
they are the minimum manual brief, not optional guidance:

> Work as a fresh, maximally adversarial reviewer. Attempt to refute the
> bead's acceptance claims and the actual PR implementation. Render an
> honest verdict: use **Verdict REFUTED.** only if you find at least one
> genuinely serious defect. A clean **Verdict NOT REFUTED.** after a real
> sweep is a successful review, not a failed one; never escalate a minor
> issue to justify the effort.
>
> After the first blocker, ask whether fixing it could plausibly moot your
> other findings.
>
> - If yes because the defect is a design-level flaw or the wrong contract,
>   stop enumerating instances. Report the defect class with two confirming
>   executions, identify the unswept surfaces, and recommend a design pass.
> - If no because the design is stable and the defects are point-local,
>   sweep exhaustively and report the complete list.
>
> Exhaustive mode does not lower the severity or evidence bar and is never
> license to pad the report with nitpicks. Every verdict must include
> `## Coverage`: state which areas were fully swept, which were not, and why.
>
> Once a blocker's core claim is guarded, a residual precision-hygiene
> finding about the new check freezes that check's contract. Replace it with
> the simplest sufficient contract or split it out; never extend the new
> check in the current PR.
>
> For a guard-shaped finding, require rework at the narrowest choke point
> that covers the whole class and probe a sibling path. Apply the same "what
> else makes it pass?" interrogation to guard-shaped specifications before
> dispatch.
>
> For security surfaces, frame briefs, probes, and findings as
> rejection-contract or correctness-invariant assertions: "verify the
> validator REJECTS X"; "only canonical encoders can produce a valid wrapper,
> and a value not produced by them is refused at the sink." Use this
> vocabulary throughout.
>
> - A blocker requires an executed failure or a byte-level demonstration of
>   the claimed failure itself on a realistic deployed path. A flag difference
>   or serialization difference alone does not demonstrate a downstream
>   failure. When relying on documentation for external-service behavior,
>   execute the claimed external-service outcome against a representative
>   real target; otherwise self-grade to a concern. Speculation never blocks;
>   a finding without executed or direct byte-level evidence self-grades to a
>   concern.
> - Every finding must include a **Threat model** stating who can trigger it
>   and from where. A path reachable only by a trusted producer self-grades to
>   a concern.
> - After cycle two, a new finding may block only if it belongs to a previously
>   unadjudicated class. Otherwise identify it as follow-up work rather than a
>   merge blocker.
> - For corpus- or file-reading code, include a cwd-variance probe.
>
> Include the required `## Coverage` statement before ending every verdict
> with `## Probes`; list the commands or inspections actually performed and
> their outcomes, including why any broader gate was necessary.

## Field hazards

- **A — provider content-filter trap.** Certain action-verb framing caused six
  occurrences in one field day: a reviewer wedged mid-run with no verdict.
  This is not limited to security briefs. Use a remedy ladder: state checks
  as correctness invariants and use "exercise"; for authentication or
  credential subject matter, describe checks mechanically with no domain
  vocabulary. A wedged pane does not recover by re-prompting. Close the
  workspace and use a fresh pane.
- **B — goal-language neutrality.** Its canonical wording lives in `ab-xuz`
  amendment 10 and is quoted in the Manual reviewer brief above. Reuse it;
  do not fork it.
- **C — author gates are not reviewer gates.** Never copy focused reviewer-gate
  trims into author briefs; doing so cost a red CI on an already-reviewed PR.
- **D — reviewer-filed blocking defects.** When a reviewer invokes the
  blocking-defect carve-out, fold that work into the rework bead unless the
  finding outlives the PR.
- **E — verify before accepting.** Check a blocker's cited mechanics yourself.
  Reviewers have been structurally right and detail-wrong.
- **F — durable concern disposition.** The section 4 transaction owns this:
  dispose of every accepted concern in the same operation, never later.

## 6. Panes, waiting, and babysitting

- Do not babysit dispatch prompts. The engine recovers codex startup paste
  races itself (baseline-relative meter comparison, bounded re-sampling).
  If a lane is genuinely stuck, the drain report says so — trust the
  report and the tracker, not a pane read.
- Pane text is diagnostics only, for machines and for you. A pane can show
  its prompt apparently unsent at 0% context while the work is in flight.
  Liveness comes from `herdr agent list` status and tracker state.
- Long-lived `herdr agent wait` subscriptions can hang past the transition
  they watch (observed: 25 minutes past idle). Prefer bounded loops of
  fresh probes over one armed wait.

## 7. The merge path

- Repo has a merge queue with `adversarial-review` required: after the
  drain flips the status, enqueue (`gh pr merge <n> --merge`, or GraphQL
  `enqueuePullRequest` if gh refuses); the queue re-validates on a
  merge-group commit and performs the merge itself. The engine posts the
  status on merge-group commits.
- Repo not configured: present the accepted PR to the operator. Do not
  merge without an explicit grant, and never configure the repo to make
  merging possible — that is the operator's act, surfaced as a request.

## 8. Close gate — before any `br close`

- The bead's test spec is implemented as written: open the named test file
  and confirm the named assertions exist. A test that touches the module
  without asserting the specified behavior does not satisfy the spec.
- The closing comment's claims match checked-in artifacts. Name files and
  numbers you verified this session, not remembered ones.
- Cross-references to other beads are current — re-check IDs written
  before later beads were filed.
- A bead claiming an end-to-end behavior is tested through the PUBLIC
  entry point, not a helper. Helper-level evidence can pass while the
  public path contradicts the claim — mb-h2x0 was falsely closed that
  way and cycle-12 review caught it.
- Before accepting any hardening-bead closure, grep the new and changed
  tests for `monkeypatch`, `setattr`, stubs, or no-ops of the guard
  functions and constants. One worker neutralized failing guards in
  tests three times across cycles while its closing comments claimed
  the guards were active.
- A consumer-path claim ("X is unaffected") requires the full caller
  graph, not the paths already in view. An unaffected claim once missed
  live rollup endpoints and the operator exposed it in one message.

## 9. Durable state — during and at session end

- Commit `.beads/issues.jsonl` at natural points and push. Unpushed
  tracker state strands the lifecycle for every other session.
- Never commit tracker bookkeeping onto an active worker's branch. The
  main checkout belongs to whichever worker owns the lane; while a
  main-checkout lane is live, orchestrator tracker commits go through a
  separate worktree or wait for the worker's report. A market-brief
  orchestrator commit landed between worker commits on
  `feat/mb-o8p7.5-reader-widening` and contaminated that branch's
  history.
- In the abacus repo only: after any engine-source PR merges,
  `cargo install --path .` before the next drain.
- An announced artifact that is not committed does not exist. Before
  ending a session, verify every handoff document, report, or file you
  said you wrote is present in the tree and pushed — a session lost a full
  day's context to a handoff doc that was announced and never written.
- Discovery goes to `jot`; blocking defects get an immediate bead. Never
  curate the jot queue yourself — `/jot-review` is operator-invoked.

## 10. Escalate, don't improvise

To the operator: repo configuration of any kind, merge grants, anything
posted under their identity whose meaning is acceptance or authorization,
reversals of recorded decisions (ADRs, north star), and any situation where
following this skill and following the engine's behavior conflict — that
conflict is evidence, capture it first.

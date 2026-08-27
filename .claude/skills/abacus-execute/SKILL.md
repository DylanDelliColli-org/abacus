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

Three seats. Do not do another seat's work.

- **Engine** (`abacus run` / `abacus drain` / `abacus land`): dispatches
  workers into lanes, classifies every settle, recovers paste races, launches
  and reaps adversarial reviewers, parses adjudications, posts the
  `adversarial-review` commit status (PR head and merge-group commits), reaps
  merged lanes. Never hand-perform these; if the engine misbehaves, capture
  evidence and escalate — do not build a manual workaround loop.
- **Orchestrator** (you): read verdicts, adjudicate them, reopen beads for
  rework, relay sandbox-denied verdict posts, keep the tracker committed and
  pushed, escalate anything identity-bearing or configuration-shaped.
- **Operator** (the human): repository configuration (rulesets, branch
  protection, merge queues, CI workflows — ADR 0003 forbids the engine and
  you from mutating these), merge grants, auto-merge opt-in via `land`,
  jot-queue curation, and anything recorded under their identity that they
  have not explicitly delegated.

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

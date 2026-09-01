```doc-meta
role: evidence
lifecycle: active
```

# Reviewer model/effort selection experiment — observation record

Dated 2026-08-31/09-01. Bead `ab-ljn.4`. Seven waves, 30 review contexts,
all blind, all file-deliverable. Ground truth: the byte-exact adjudication
records on PRs 45–50, the ADR 0006 D8 shadow-trial reports, and orchestrator
E-checks. This record preserves the data the decisions in ADR 0007 rest on;
the raw reports lived in gitignored `target/abacus-tmp/reviews/` and are not
retained.

## Design

Two experiments, one program. **Model/effort selection:** re-run historical
review briefs blind at pinned heads across cheaper configs, scored on
adjudicated-blocker recall and manufactured-blocker rate. Leak controls:
PR comment trails and tracker comments forbidden; bead descriptions inlined
into briefs; nothing posted. **Methodology prescription** (operator
hypothesis mid-experiment): identical runs with the brief additionally
prescribing the *method* (fake-shim execution harness for correctness;
concept-to-guarantee trace, claims-vs-tree/parent-diff audit, test-mapping
procedure, execute-don't-estimate for simplicity), testing whether the
capability delta is prompt-shaped.

Configs: `gpt-5.6-sol` at medium; `gpt-5.6-luna` at high and xhigh — all
selected via the herdr `AGENT_ARG` passthrough
(`herdr agent start <n> --kind codex --pane <p> -- --model <m> -c
model_reasoning_effort=<e>`), verified live by process argv on every wave
(the OC-9 smoke test). Baseline: the historical Sol-tier reviews and the
Sol-tier shadow-trial reports.

## Correctness arm — recall vs the adjudicated record

Heads: 46-c1 (`cd872a4`, 2 adjudicated blockers), 48-c1 (`c9c3155`, 1
design blocker) for recall; 46-c3 (`434d7d7`), 48-c3 (`c7a44b0`), both
adjudicated NOT REFUTED, for neutrality.

| Config | Blocker recall | Manufactured | Wall (4 heads) |
|---|---|---|---|
| Sol-tier (historical baseline) | 3/3 | 0 | 10–20 min per review |
| Sol-medium, plain brief | 2/3 | 0 | 6m52s total |
| Luna-high, plain brief | 0/3 | 0 | 5m53s total |
| Luna-xhigh, plain brief | 1/3 | 0 | 6m50s total |
| **Sol-medium + methodology** (46-c1) | **2/2 on that head**, split into both executed halves at the adjudication's own granularity | 0 | 11m14s for 3 configs |
| Luna-high + methodology (46-c1) | 1/2 | 0 | — |
| Luna-xhigh + methodology (46-c1) | 1/2 | 0 | — |

The decisive pattern: every plain-brief miss was a blocker whose original
discovery required building and executing a fake-shim harness; the
statically-readable parser blocker was caught by two of three cheap
configs. The evidence *bar* ("a blocker requires an executed failure") did
not elicit the executed *method*; prescribing the method closed the gap
fully for Sol-medium and half for Luna. Luna's residual is model-shaped.

**Neutrality control became a discovery.** Sol-medium + methodology on
"clean" head 46-c3 returned REFUTED — and orchestrator E-check confirmed a
**true positive the historical Sol-tier review missed**: a bead deferred
between the sweep's list snapshot and its authoritative `br show` probe
crashes the drain (`classify_bead_status` has no `deferred` arm,
`src/lib.rs:144-151`; executed trace `exit 1: unsupported bead status
"deferred"`), verified live at then-current main and captured to the jot
queue. Consequence: zero manufactured findings were observed at any tier
anywhere in the program, and the neutrality cell is vacated rather than
failed.

## Simplicity arm — vs the Sol-tier trial baseline

Targets: abacus PR 48, mbp PR 61 (the known-bloated 21-cycle arc).

| Config | Plain brief | + Methodology |
|---|---|---|
| Sol-medium | 48: missed the COLLABORATOR merge regression (the top finding). 61: caught the parity-framework removal, nothing else | 48: **caught COLLABORATOR**, with executed test runs. 61: mutation-matrix trim with a **measured** run (75 passed, 1.32s, max-RSS recorded), 21 duplicate cases removed with per-case surviving-guarantee |
| Luna-high | 48: missed COLLABORATOR. 61: 4 proposals incl. two shadow-class items, but missed parity-framework | 48: caught COLLABORATOR. 61: **caught the parity-framework removal** |

Every wave-4 top-value miss closed under methodology. The residual vs the
Sol-tier shadow (label-honesty findings, unconditional suite measurement,
deepest enumeration) did not fully close and motivated the v2 methods.

## Full three-seat stack — Sol-medium + methodology (wave 7)

Six contexts, PRs 46 and 48, one seat each of correctness (shim
methodology), simplicity (methodology + scope note routing test depth to
the specialist), and test-quality (first execution of the draft v1
contract: behaviour map, distinctness map, label-honesty audit,
reachability check, deleted-assertion accounting, unconditional
measurement). **Wall: 9m25s for the entire stack** vs 10–20 min for one
Sol-tier review.

| Seat | PR 46 | PR 48 |
|---|---|---|
| Correctness | REFUTED with exactly the deferred-crash blocker, independently rediscovered from a brief that did not name it; nothing else | NOT REFUTED, zero blockers, matching the cycle-4 record; correctly silent on the COLLABORATOR issue (test seat's territory) |
| Test-quality | Partial: legacy-alias and parser-kind classes caught; 32-boundary and open-agent classes missed (~2–3 of the shadow's 5 groups) | Both shadow headliners caught: the COLLABORATOR coverage loss and the duplicate-proof pair, with measured runs |
| Simplicity | Scope carve held: zero test-depth content | Same |

Composition: no carve leaks, no duplicate findings across seats, 216 total
report lines across six reports.

## Shadow-trial context (2026-08-31, bead `ab-ljn.3`)

The prior experiment this program built on: 7 PRs × paired blind contexts
(instructed simplicity vs a dedicated test-quality shadow). The D8
promotion threshold (two independent PRs with material shadow-only test
defects) was met five times over (abacus 46; mbp 61, 72, 73, 74 clear;
abacus 48 and mbp 64 partial), with recurring classes: duplicate
behavioural proof, non-load-bearing assertions, coverage loss via
fabricated fixtures and untested branches. The crowding check was clean.
It also surfaced a live coverage regression in merged main (the
COLLABORATOR drain integration lost in PR 48's conflict resolution),
independently found by both trial contexts.

## Standing caveats

Small n throughout: two refuted heads, two accepted heads, two simplicity
targets, one stack run per PR. The test seat's deepest enumeration trails
the Sol-tier shadow. Sol-high was never tested (the one config between
Sol-medium's plain-brief misses and the Sol-tier record). All conclusions
inherit these limits; the missed-blocker tripwire in ADR 0007 is the
standing correction mechanism.

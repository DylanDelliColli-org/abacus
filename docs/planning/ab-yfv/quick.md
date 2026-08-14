# Quick run: lane duration in `abacus run` output

- **Epic:** ab-yfv — lane duration reporting (this file is its planning state)
- **Tier:** Quick, operator-confirmed 2026-08-14. Rationale: the ask is
  well-specified, has no unknowns to research, introduces no new interface or
  contract (one output line), and lands in a single bead.
- **Live-run context:** this is the first live run of `/abacus-plan`, measured
  by ab-qmc.3 (the bead that tests the planning flow against its success bar).

## Frame

**Outcome.** `abacus run` reports each lane's wall-clock duration in its
outcome line, for example: `bead ab-irn is closed; worker completed in 4m07s`.
This is the measurement the north-star kill criterion needs — machinery must
beat one or two vanilla coding-agent sessions, and today nothing records what
a lane costs.

**Scope.** Start timing when the lane opens (immediately before the herdr
worktree call) and stop when the outcome is known. Print the duration on
every outcome path: completed, incomplete, never-engaged, and the error path
when a lane leg fails after opening. The empty-backlog path prints no
duration because no lane opened.

**Non-goals.** No persistence of timings, no aggregate statistics, no
per-substep breakdown, no engine timing store. One line of output per run.

**Vetoable assumptions** (stated, not laundered):

1. Timing starts at lane-open, not at prompt-submit — the lane's full cost
   including worktree and agent startup is the number the kill criterion
   wants.
2. Failure paths also carry the duration ("failed after 12s") — a cheap lane
   failure and an expensive one are different facts.

**Prerequisites.** None — explicitly asserted. The implementation touches
only the engine binary's output; no bead must land first.

## Test strategy

Current measured full suite: **1.48s** against
`FULL_SUITE_WALL_CLOCK_BUDGET_SECONDS = 30`, leaving ~28.5s remaining.

| Case | Layer | Extends existing? | Assertion | Estimated cost |
|---|---|---|---|---|
| Duration formatting (seconds, minutes+seconds boundaries) | unit | new `#[test]` in existing `src/lib.rs` module | `format_lane_duration` renders `42s`, `4m07s`, `0s` | ~0.01s |
| Failure path carries duration | integration | extends the existing no-herdr-on-PATH test in `tests/br_roundtrip.rs` | stderr matches `after \d+` on the lane-leg failure | ~0.1s (reuses the existing binary invocation) |

Estimated addition: ~0.1s. Post-change suite stays under 2s — no budget
pressure. Happy-path duration output is exercised by the live dispatch that
drains this bead (the outcome line is read by the orchestrator), not by a
mocked herdr — consistent with integration-at-real-seams.

## Proposed bead (1 of 1)

**Title:** `abacus run` prints lane wall-clock duration in every outcome line

**Description (as it will be created):** implementation in `src/main.rs`
`cmd_run`: capture `std::time::Instant` immediately before the herdr worktree
create call; compute elapsed at every outcome exit (the three
`BeadOutcome` arms and the error path of any lane leg after the timer
starts). Add `format_lane_duration(secs: u64) -> String` in `src/lib.rs`
rendering `42s` under a minute and `4m07s` at or over a minute. Print
`… in <duration>` on success lines and `… after <duration>` on failure
lines. Test spec: unit test in `src/lib.rs` for the three formatting shapes
(0s, 42s, 4m07s); extend the existing PATH-restricted integration test in
`tests/br_roundtrip.rs` to assert the failure message matches `after `.
Write the failing tests first. Verification: `cargo test`, `cargo clippy`,
`cargo fmt --check`.

**File footprint:** `src/main.rs`, `src/lib.rs`, `tests/br_roundtrip.rs`

**Group tag:** none — single bead, no bundle candidates.

## Open questions

None. Both stated assumptions are vetoable at this gate; if either is
rejected the bead text changes before creation, not after.

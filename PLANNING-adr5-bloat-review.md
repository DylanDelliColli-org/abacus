# Bloat review: ADR 0005

1. **Defer — the adversarial-review cluster.**
   - **What:** Cut D3, D4, D8, `AwaitingReview`/`ReworkRequested`, D5’s warm-rework and branch-resurrection machinery, and their dependent status, test, and consequence text.
   - **Why:** The North Star says, “A backlog of N ready beads drains to closed overnight across two repositories with **zero operator interventions**” and “Acceptance happens inside the agent team — a reviewer accepts, the bead closes — while the human review gate sits at the merge boundary.” This design instead waits for operator adjudication before acceptance or rework. Revive when an overnight run can produce and adjudicate rework entirely inside the agent team.
   - **Cost of cutting:** The MVP loses systematic refutation, warm rework, and GitHub status signaling; defects may survive to morning review.

2. **Delete — D7’s extraction-first prerequisite.**
   - **What:** Drop the behavior-preserving `dispatch_cycle` decomposition from this delivery.
   - **Why:** “The engine exists to buy throughput under autonomy.” Moving existing behavior before landing required drain states produces no observable MVP outcome.
   - **Cost of cutting:** `dispatch_cycle` remains roughly 140 inline lines and harder to maintain; the success condition loses nothing observable.

3. **Shrink — the Test contract’s implementation bookkeeping.**
   - **What:** Remove the fixed 28/17/11 counts, zero-new-files rule, real-br cap, projected runtime, and unrelated `br_roundtrip` retry.
   - **Why:** “A backlog of N ready beads drains to closed overnight across two repositories with **zero operator interventions**.” Test quotas, file placement, forecasts, and an unrelated flake repair do not make that outcome work.
   - **Cost of cutting:** Coverage shape becomes less prescribed, runtime has no forecast, and the known flake remains.

4. **Defer — D6’s new `abacus run` exit code.**
   - **What:** Cut exit code 3 and the single-run wrapper contract; retain only the drain behavior under decision here.
   - **Why:** The stated success condition is an overnight backlog drain across two repositories, not richer signaling for the single-bead command.
   - **Cost of cutting:** Existing wrappers cannot distinguish a parked settle from engine failure. Revive when a real wrapper needs that distinction.

# Bloat review — ADR 0004

1. **Shrink D2 and D9 to the first runnable backlog.** Import only an operator-selected `N` of ready beads; cut the 351-record historical/deferred import and the 117-bead SABLE classification from onboarding. Why: “A backlog of N ready beads drains to closed overnight across two repositories with **zero operator interventions**.” Cost: deferred, in-progress, and recent history stays archive-only; continuity in `br` is lost.

2. **Defer D8, target-repo docs-doctor conformance.** Revive when unmanaged documentation actually blocks a lane/review or a managed document must change. Why: the success condition is “The final clean PR(s) are ready for operator review in the morning.” Converting 25 documents does not establish that outcome. Cost: documentation remains unconformed and three content decisions stay unresolved.

3. **Shrink D10 to historical rationale; drop two-session seat-signoff as an onboarding requirement.** Revive only after an authority collision invalidates or blocks an action. Why: “The engine exists to buy throughput under autonomy. If the orchestration, verification, and coordination cost more wall-clock than the naive baseline delivers, no other property redeems it.” Cost: less independent cross-checking and weaker authority provenance.

4. **Defer D6’s successor-before-first-drain gate.** Keep land mode off; revive when the operator opts this repository into overnight merging. Why: “Autonomy ends at the PR, except on repositories opted into overnight merging.” Cost: produced PRs may wait at the human merge boundary, which the MVP explicitly permits.

5. **Delete the exhibit’s “Non-Interactive Shell Commands” and “Session Completion” sections.** Push, PR creation, clean-worktree verification, and close-last already appear in the lane protocol; the remaining shell and branch-cleanup advice is generic. Why: “ABACUS orchestrates agents against a backlog. It is not a library for arbitrary agent workflows.” Cost: workers lose reminders about interactive aliases, rebasing, stashes, and stale remote branches; no required PR artifact is lost.

6. **Shrink D4 by deleting mandatory red-first evidence recording and PR-review enforcement.** Keep the test-first instruction and matching gates. Why: “Acceptance happens inside the agent team — a reviewer accepts, the bead closes — while the human review gate sits at the merge boundary.” Passing evidence establishes acceptance; recording test chronology does not. Cost: no auditable proof that the test failed before implementation.

7. **Defer open question 4, the `conftest.py` import-resolution guard.** Revive if a compliant step-zero run still imports the main checkout or a review accepts such a leak. Why: “The machinery makes execution slower than simply running one or two vanilla coding-agent sessions” is the kill criterion; the mandated assertion already covers the first drain. Cost: leak detection remains procedural rather than automatic.

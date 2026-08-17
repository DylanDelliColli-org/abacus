# ADR 0004 bloat-cut positions — resident-session counterparty input

Review-only input from the market-brief-package resident session
(2026-08-17), filed verbatim in substance for the operator's disposition
and the spec validator's context. The orchestrator's own per-cut context
was reported to the operator separately; dispositions are the operator's.

| Cut | Resident position | Core argument |
|---|---|---|
| 1 | Shrink the **drain**, not the import | Import and drain scope are separable; import cost is sunk (backup verified, export double-verified, transform specified). Ready-only import would orphan the live `xiv8r` diff (the bead is the sole record — the 08-13 shift report deliberately does not duplicate it) and silently vanish `xllia`'s pending operator decision. Drain-set selection achieves the reviewer's stated goal at zero import cost. |
| 2 | Agree (defer docs-doctor conformance) | ~2h, gated on a commit-authority grant that does not exist; the three content decisions inside it are the resident operator's and need no doctor run. |
| 3 | No view | Orchestrator-fleet governance. |
| 4 | Agree; blocker withdrawn | With land mode off, PRs waiting at the human boundary is the permitted MVP shape; the sable-merge-gate successor is the resident repo's governance question, not this ADR's. |
| 5 | Partial: drop Session Completion, relocate the branch-prune line into the lane protocol; **keep** Non-Interactive Shell | Session Completion is genuinely duplicative of the lane protocol except stale-branch pruning, which is fleet-scale (one lane branch per bead). Non-Interactive Shell prevents the worst unattended failure shape — a lane hung indefinitely on an `-i`-aliased prompt, holding its slot and producing nothing overnight. Generic AND load-bearing. |
| 6 | **Reject — keep red-first recording** | The reviewer's "passing evidence establishes acceptance" is a category error: a test written after the implementation and never observed to fail can pass vacuously, and passing evidence is exactly what a vacuous test also produces. With tdd-gate retired and non-firing on br, the recorded red-first note is the only surviving artifact that TDD happened; an unobservable instruction to an unattended fleet is a preference, not an instruction. Cost of keeping: one bead comment. |
| 7 | Agree, reframed | Defer the conftest guard to the resident backlog, but as "procedural for now, mechanism later" — the contract assertion covers the first drain only if lanes execute it; it is a line, not a mechanism. |

The resident session is routing the same positions to its own operator.
Freeze respected throughout; no codebase actions taken by either session.

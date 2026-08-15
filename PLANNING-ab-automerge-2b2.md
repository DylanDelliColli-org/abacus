```doc-meta
role: planning-inflight
lifecycle: inflight
```

# PLANNING — ab-automerge-2b2 (PR validation and auto-merge machinery)

**Tier: FULL** — confirmed by operator 2026-08-14.

**Rationale:** the ask introduces new contracts (merge policy, validation
gate, risk-tier classification), carries real unknowns (moving-base
dynamics during a multi-bead run, validation semantics for an unattended
merge), amends the thesis (NORTH-STAR non-goal 3 and the success
condition currently end autonomy at the PR), and has a blast radius
spanning the engine, the worker protocol, at least one ADR, and the
north star. Every Quick criterion fails.

**Operator direction seeding the ask (2026-08-14):** without engine-side
validation and merging, the base moves constantly during an overnight
multi-bead run. Beyond PRs-waiting-in-the-morning, the operator wants an
autonomous mode for lower-risk or lower-complexity projects where
auto-merge throughput matters more than morning review.

Substages append below: FRAMING, RESEARCH, ARCHITECTURE, TEST-STRATEGY,
RECORD, DECOMPOSITION. This file is deleted at handoff; git history is
the archive.

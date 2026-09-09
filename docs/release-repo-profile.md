```doc-meta
role: contract
lifecycle: active
```

# Repo-owned execution profile template

Use only after [explicit activation](release-pilot.md). Adapt the block below
into one target-repo document, for example `docs/execution-profile.md`, and point
the repo's AGENTS.md and CLAUDE.md at that same file when both exist. This template
does not activate this repository or any other task. Do not install global rules
or copy the shared contracts into the repo.

The chief populates engineering facts from the repo and maintains them as they
change; use existing docs where they already answer the question. The operator
supplies product direction and any genuinely new authority, not approval of
every command or field. Omit irrelevant details. This is an example shape, not
a schema, mandatory checklist or second tracker.

> Shared execution baseline: [read-only packet path and full commit SHA].
> Read release-pilot.md and the applicable chief/reviewer contract under its docs/.
> Activation: [target task bead containing scope, pinned product outcomes,
> explicitly locked constraints, integration branch and approved overrides].
> Repo context: [existing architecture/conventions/setup documents].
> Verification: [actual unit, real integration and full-suite commands; safe
> startup/smoke journey; any required environment or test data].
> Coordination: [Herdr workspace, lane naming and tracker binding; current
> model/effort/tier or other resource constraints].
> Runtime: unrestricted for activated pilot sessions, as defined in the shared
> baseline; [provider-specific supported launch arguments and any host constraint].
> Local resources: [task-owned runtime/cache/data paths and shared-service limits].
> Release: [required merge gates and compliant route; existing merge/deploy
> authority or none; recovery and deployed smoke context when deployment is in scope].

Facts and commands here refine the shared baseline; they do not weaken its
product/engineering boundary or independent acceptance. They cannot grant
permissions that the activation or higher-priority instructions withhold. If
existing repo instructions conflict with the approved execution mode, resolve
the specific conflict explicitly; do not silently choose whichever is convenient.

Pin the profile revision/snapshot with the activated task and pass it to every
delegate, including reviewers and resumed agents. Keep operational changes
durable and propagate the new reference to affected agents. Never silently
replace the recorded acceptance source or shared baseline when updating local
setup details. Reuse this profile across later tasks only when those tasks are
explicitly opted in; its existence is not repo-wide activation.

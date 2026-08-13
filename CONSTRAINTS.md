# Constraints — measured findings

These four findings were paid for with real failures in `abacus-v1` and its
predecessor sessions. Source: `SHIFT-REPORT-2026-08-13-CLAUDE.md` §7. They are
evidence transport, not design: nothing here prescribes an implementation, and
nothing may be added to this file without a measured failure behind it.

## 1. `br`, not `bd`

At 11 concurrent claimants, `br` served 879 reads with zero timeouts
(p50 51ms). `bd` recorded 15/15 read timeouts at similar width. The work
store is `br`; do not reintroduce `bd` on any path the engine touches.

## 2. Provider identity binds to every execution, not to startup

A gate that checks the provider once at launch was defeated three separate
ways, each reproduced live: a cached verdict, a held inode, and an ambient
staging root. If it matters which provider executes, the binding must be
checked at the execution, every time.

## 3. The worker launch environment must carry bead and attempt

A context-lost worker that knows only "I am a codex pane" cannot enumerate
its own records. The launch path must carry the bead identity and the
attempt into the worker's environment, or recovery after a wipe means a
human reconstructing what the lane was for. (MVP carriage today: branch
name `lane/<bead-id>` plus the dispatch prompt.)

## 4. Crash recovery is first-class on this host

The operator's machine crashes. This is a live, recurring condition, not a
tail risk: "that only breaks if the host dies at the wrong moment" is not a
mitigating argument here. Anything stateful must either survive a crash or
be cheaply reconstructible after one.

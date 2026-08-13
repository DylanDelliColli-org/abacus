# Agents

You are in the **abacus** repository: an execution engine over `br` (work
state) and Herdr (agent orchestration). One page; follow the links, they are
the authority.

## Orientation

- **`NORTH-STAR.md`** is the standard every proposal is judged against. Read
  it before proposing anything.
- **`CONSTRAINTS.md`** carries the four measured findings. They are paid-for
  evidence; do not relearn them the expensive way.
- **Work state lives in `br`.** There is no other tracker and no TODO lists.

## Working a bead

```sh
br ready                      # what is available
br show <id>                  # your full scope — the description is the spec
br update <id> --claim        # claim before you start
br close <id>                 # only after the work is verified
```

Never use `br edit` (opens an editor and hangs you). Update fields with
`br update <id> --description "..."` / `--notes "..."`.

## Lanes

- A worker lane is a git worktree under `~/.herdr/worktrees/abacus/` on a
  branch named `lane/<bead-id>`. Do all work inside your own lane.
- Write the failing test first; implement until green; run the full suite
  (`cargo test`), plus `cargo clippy` and `cargo fmt --check`.
- Commit and self-push your lane branch: `git push -u origin lane/<bead-id>`.
- **Autonomy ends at the PR.** Never merge to `main`; the operator reviews
  at the merge boundary.

## If you are lost

Your bead id is in your branch name (`git branch --show-current`) and your
dispatch prompt. `br show <that id>` restores your scope. If you cannot
proceed, say BLOCKED and why, and stop — do not improvise scope.

```doc-meta
role: contract
lifecycle: active
```

# Agent Instructions

This repository uses **br (beads)** for issue tracking; work state lives in
`.beads/issues.jsonl`, which is committed and rides `git push` — there is no
separate tracker remote and no push command for it. Worker lanes dispatched
by abacus operate under the contract below; it applies equally to any agent
working in this repository.

## Quick Reference

```bash
br ready              # Find available work
br show <id>          # View issue details
br update <id> --claim  # Claim work atomically
br close <id>         # Complete work
br q "<title>"        # Quick capture
```

Do NOT use TodoWrite, TaskCreate, or markdown TODO lists — `br` is the only
tracker. Persistent operator policy lives in the standing-directives document
(interim location recorded in the migration bead; its final name and survival
are under operator review) — `bd remember` no longer exists here.

## Worker lane protocol

1. Your bead is claimed to your lane; `br show <id>` is your full scope.
2. Write the failing test first. Record red-first evidence (the failing run)
   in the bead notes before implementing. This is checked at PR review.
3. Implement until green, then run every matching gate from the table below
   — starting with step zero.
4. Commit source and test changes only; push your lane branch; open a PR
   with `gh pr create` (title carries the bead id).
5. `br close <id>` is your final act, after the push succeeded and the PR
   exists. Verify the worktree is clean.
6. **Never merge.** Merging into `llm-integration` has exactly one
   sanctioned path, and it is not yours.
7. If you cannot proceed, make the state durable BEFORE stopping:
   `br comments add <id> -m "BLOCKED: <why, and what you tried>"` — a
   blocked lane never reaches `br close`, so stdout is the only other
   trace and stdout gets lost. Then say BLOCKED and stop.

## Step zero — import-leak discipline (before any pytest)

Most Python packages here (`headless_eval`, `location_briefing`, all six
SDKs) are **editable-installed with absolute paths pinned to the main
checkout**. Python consults `sys.path` (cwd first) before those editable
mappings, so imports resolve to YOUR code only while your cwd contains the
package. The leak: any invocation whose cwd does not contain the package
resolves to the **main checkout** — your change never executes and the
green is about code you didn't touch.

1. **Run every Python gate from your worktree root. Never `cd` into a
   subdirectory first.** In particular, do NOT mirror `test.yml`'s
   `working-directory: location-briefing` + `../headless_*` invocation of
   the sibling gate — CI is a single checkout, so that is safe there and
   unsafe for you. Two legitimate exceptions: the backend gate runs from
   `location-briefing/` (safe — `location-briefing/src` sits under that
   cwd), and the frontend gate runs from `location-briefing/frontend/`
   (safe — npm executes no Python imports). Return to the worktree root
   before any other Python gate.
2. For every top-level package your diff touches, assert it resolves
   under your own tree (the oracle — run it, don't reason it out).
   From the worktree root, for the sibling trees:

   ```bash
   python -c "import headless_eval, pathlib as p; f=p.Path(headless_eval.__file__).resolve(); assert str(f).startswith(str(p.Path.cwd().resolve())), f'IMPORT LEAK: {f}'"
   ```

   For `location-briefing` trees, run this from `location-briefing/`
   (same cwd as the backend gate). The distribution's importable module
   is literally named `src` — the editable finder maps only that name;
   `import location_briefing` does not exist and will always
   ModuleNotFoundError:

   ```bash
   python -c "import src, pathlib as p; f=p.Path(src.__file__).resolve(); assert str(f).startswith(str(p.Path.cwd().resolve())), f'IMPORT LEAK: {f}'"
   ```

   Because `src` is a generic name, ANY directory containing a `src/`
   shadows it — so a tripped assertion can mean wrong checkout OR wrong
   project entirely; read the resolved path in the failure message to
   tell which before choosing a remedy.
3. If an assertion trips anyway: prepend the directory that CONTAINS
   the module — for the sibling trees your worktree root, for `src`
   the worktree's `location-briefing/` — e.g. from the worktree root
   `PYTHONPATH="$PWD:$PWD/location-briefing"` (add the parent of the
   SDK module you touched, if any) — and re-run the assertion;
   `PYTHONPATH` entries deterministically precede the editable
   mappings. Trust the gate only if the re-run assertion passes under
   the same environment.
4. Still leaking: **BLOCKED** per protocol step 7. Do NOT
   `pip install -e` from a lane worktree — that repoints the shared
   environment at a worktree that will be reaped, breaking the main
   checkout and every other lane.

Note: `location-briefing/tests/smoke/test_sdk_install.py` does NOT cover
this — it proves "not a frozen site-packages install," and a foreign
checkout satisfies it. It establishes nothing about whether imports are
YOUR code; only the step-zero assertion does.

## Test gates — select by changed path, never "the full suite"

A bare `pytest` from the repository root is **invalid**: 58 collection
errors from colliding per-SDK `tests/` packages (market-brief-package-mohr4).
If you see that wall of errors you ran the wrong thing — it is not
"pre-existing red." Select gates by what your diff touches:

| You touched | You must run |
|---|---|
| `headless_intelligence/` or `headless_eval/` | `python -m pytest headless_intelligence/tests headless_eval/tests` (~28s, hermetic, offline) |
| `location-briefing/backend/` or `location-briefing/src/` | the backend-unit gate, from `location-briefing/`: `python -m pytest --ignore=tests/retired --ignore=tests/integration --override-ini="asyncio_mode=auto" --cov=backend/app --cov=src --cov-report=term-missing --cov-fail-under=50` |
| `location-briefing/frontend/` | `npm run lint` and `npm test` from `location-briefing/frontend/` |
| any `.md` file, anywhere | additionally: `python -m pytest headless_intelligence/tests/test_openai_only_architecture.py` — it scans `docs/` prose and has failed innocent-looking documents three times |
| more than one of the above | every matching gate |
| you cannot classify your diff | all three offline gates — slow beats a meaningless green |

Notes that save you a wrong conclusion:

- The backend gate enforces `--cov-fail-under=50`: adding uncovered code
  fails on **coverage**, not correctness; `term-missing` shows you which
  lines. Read the failure before debugging phantom logic errors.
- CI's backend job runs `make install-sdks` and
  `make test-smoke-sdk-install` before pytest. In a lane, do NOT run
  `install-sdks` (shared-environment mutation, see step zero); DO run
  `make -C location-briefing test-smoke-sdk-install` with the backend
  gate, remembering it complements the step-zero assertion, not replaces
  it.
- The Docker/Supabase integration suite is **exempt for lanes** — it needs
  a local Docker Supabase, a cross-repo migrations checkout, and a token.
  Do not attempt it; say so in the bead notes instead of running it. This
  is the stated exception to the unit-AND-integration rule.
- A clean pre-push hook is **not** test evidence — its test phase silently
  no-ops repo-wide (market-brief-package-i3xzk). Only the gates above count.

## Prohibitions

- **Never merge** (see lane protocol). **Never `--admin`-anything.**
- **Never force-push** — it bypasses the entire pre-push gate
  (market-brief-package-zh4gt).
- **Never run `black`** or any whole-file formatter — files are
  non-conforming at HEAD; reformatting is a merge hazard and is forbidden
  on security-critical files by standing directive.
- **Never `pip install -e` from a lane worktree** — see step zero.
- **Never push tracker state anywhere but `git push`** — `bd dolt push` is
  gone with bd; the standing chuck-only directive
  (dolt-push-chuck-only-2026-07-09) is preserved in the standing-directives
  document.
- Set `PYTHONDONTWRITEBYTECODE=1` in your environment — Docker test runs
  have repeatedly left root-owned `__pycache__` in worktrees.

## Non-Interactive Shell Commands

**ALWAYS use non-interactive flags** with file operations to avoid hanging on
confirmation prompts.

Shell commands like `cp`, `mv`, and `rm` may be aliased to include `-i`
(interactive) mode on some systems, causing the agent to hang indefinitely
waiting for y/n input.

**Use these forms instead:**
```bash
# Force overwrite without prompting
cp -f source dest           # NOT: cp source dest
mv -f source dest           # NOT: mv source dest
rm -f file                  # NOT: rm file

# For recursive operations
rm -rf directory            # NOT: rm -r directory
cp -rf source dest          # NOT: cp -r source dest
```

**Other commands that may prompt:**
- `scp` - use `-o BatchMode=yes` for non-interactive
- `ssh` - use `-o BatchMode=yes` to fail instead of prompting
- `apt-get` - use `-y` flag
- `brew` - use `HOMEBREW_NO_AUTO_UPDATE=1` env var

## Session Completion

Work is NOT complete until `git push` succeeds.

1. File beads for remaining follow-up work
2. Run the matching test gates (table above) if code changed
3. Update bead status — close finished work, update in-progress items
4. Push:
   ```bash
   git pull --rebase
   git push       # the committed .beads/issues.jsonl rides this push
   git status     # MUST show "up to date with origin"
   ```
5. Clean up — clear stashes, prune merged remote branches (a fleet minting
   one `lane/<bead-id>` branch per bead accumulates stale branches fast)
6. Verify all changes committed AND pushed; never stop before pushing

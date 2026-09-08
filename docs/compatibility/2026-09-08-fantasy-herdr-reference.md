```doc-meta
role: evidence
lifecycle: active
```

# Shared Herdr reference for the Fantasy experiment

Checked against the installed binary on 2026-09-08 with `herdr --skill` and
subcommand help. This describes tool mechanics; arm-specific strategy comes
from the leader's brief. Use explicit IDs/names and keep the user's focus.

## Environment and identity

Run `test "${HERDR_ENV:-}" = 1` before control operations. Commands address the
current Herdr server; always supply the intended repository/worktree cwd.
Use `herdr agent list` and `herdr workspace list` for live identifiers. Do not
predict pane IDs, use sidebar positions, or target another client's focused pane.
Use names prefixed `fx-a-` or `fx-b-`, at most 32 characters, unique on the server.

## Create an isolated checkout and agent

The installed syntax is:

```sh
herdr worktree create --cwd REPOSITORY --branch BRANCH --base BASE_REF --label LABEL --no-focus
herdr agent start NAME --kind codex --pane PANE_ID -- -m gpt-6-astra -c model_reasoning_effort=medium --disable multi_agent --disable multi_agent_v2 --approve-for-me
```

Replace uppercase tokens with actual assigned values. Read the pane identifier
from `.result.root_pane.pane_id`, and the actual checkout path/workspace ID from
the returned JSON. Launch only in an available shell pane. `agent start` waits
for agent readiness; if it returns blocked/not-ready, inspect before retrying.
Do not use destructive cleanup or server restarts to work around a startup issue.
The normal workspace-write sandbox and automatic approval reviewer remain active.

Every delegate's user prompt must include the experiment's Herdr-only rule,
matched configuration, own repo/worktree and task, parent identity, required
evidence, and the relevant arm's execution contract. For code work, name the
test files and require meaningful red before implementation, then green and
full applicable checks. Do not rely on a delegate inheriting the leader's chat.

## Submit, inspect, and follow up

```sh
herdr agent prompt NAME "TASK_TEXT" --wait --until working --timeout 15000
herdr agent get NAME
herdr agent read NAME --source recent-unwrapped --lines 120
herdr agent wait NAME --timeout 30000
```

Pass prompt text as a correctly quoted CLI argument, preserving literal
Markdown. Programmatic callers should use an argv array or proper shell quoting;
JSON escaping alone is not shell escaping. Never embed backticks in an unescaped
double-quoted shell argument. Use a concise prompt that points to an exact local
brief when the brief is long. Read the whole brief before acting.

`--until working` is useful to launch other independent work without waiting
for completion. From a non-working state, current Herdr requires observed work
or a blocked state shortly after prompt submission. A stalled/timeout result is
not proof that nothing was delivered: inspect before sending again. Do not
blindly resend a task or press Enter into an unknown approval dialog.

`idle` and `done` indicate readiness for input; neither proves the deliverable
is complete. `blocked` indicates a detected question/approval UI; `unknown` is
unclassified. Check committed artifacts, test output, tracker state and review
reports. If a prompt is visibly unsent at an ordinary input prompt, the logical
key command is `herdr agent send-keys NAME Enter`; verify the specific state first.
Send follow-up work with `agent prompt`; do not answer product questions or
approval prompts on the operator's behalf beyond the delegated experiment scope.

Keep waits bounded. Work on other useful tasks while delegates run, within the
two-active-delegate cap for the whole arm. Read terminal output for diagnostics;
large responses can leave alternate-screen history, so preserve important
evidence in the report paths agreed for this experiment. Both arms receive the
same report convention. A reviewer report names the exact inspected commit.

## Publish and clean up owned resources

Workers push their own branch to the assigned arm's origin. Use `gh` body-file
arguments for Markdown PR descriptions/comments. Explicitly select the correct
repository from its cwd; do not reuse the other arm's remote. Never merge to the
default branch or change repository protections in this experiment.

After checking the commit/report is retained and the agent is finished, close
only the workspace you created with `herdr workspace close WORKSPACE_ID`.
`herdr worktree remove --workspace WORKSPACE_ID` removes its checkout; inspect
the worktree and preserve work first. Do not use `--force` or broad cleanup.
Do not close the planning agent, coordinator, other arm, or unrelated repo agents.

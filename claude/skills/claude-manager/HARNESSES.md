# Harnesses

What differs between Claude Code and pi for the manager. `cm` handles the
mechanical differences (launch and resume lines, transcript locations,
pane detection); this page covers what you need to decide or know.

A managed session is one harness, all its agent panes included. A
manager spawns workers of its own harness.

## Claude Code

- Models: aliases `haiku`, `sonnet`, `opus`, `fable` (see the model list
  in your context for the current set).
  - Mostly invoking a skill, mechanical → smallest.
  - Clear-scope work → mid-size.
  - Real engineering, design, cross-system work, debugging, non-trivial
    review, and the default when unsure → strongest.
- Effort (`--effort`): `low` for mechanical or small, `medium` for
  moderate, `high` up to `xhigh`/`max` for large, ambiguous or hard.
  Effort follows size even on a strong model; a small task at low effort
  doesn't overthink. Blank or thin signal: strongest model, `medium`.
- Resume keeps the model but not effort; `cm` replays the recorded
  effort for the primary worker.
- Watch: `Monitor` on `cm watch --ignore-pane <your pane>`.
- Task list: yes.
- Subagents with a model choice: yes.
- Worker skill command: `/claude-manager-worker <mode>`.
- Busy in a capture: a spinner line with `(<elapsed> · ↓ <n> tokens)`.
  Idle: a past-tense line (`✻ Baked for 4s`) above the input box.
  Blocked: a dialog with numbered options and no input box.

## pi

- Models: read `defaultModel`, `defaultThinkingLevel` and
  `enabledModels` from `~/.pi/agent/settings.json` once per invocation
  (`pi --list-models <search>` expands a glob). The set changes often, so
  there is no fixed table:
  - Most work → the default model.
  - Design, debugging, review → the strongest enabled model.
  - Mechanical, mostly invoking a skill → a small or local model.
  - If the names don't make the ranking obvious, ask the user.
- Effort is pi's thinking level (`off`, `minimal`, `low`, `medium`,
  `high`, `xhigh`, `max`), passed as `--effort` to `cm`. Same sizing as
  Claude.
- Resume restores both model and thinking level.
- pi's process shows only `pi` in `ps`, so a pane's session id can't be
  read from the process. Keep worker lines complete; `cm reconcile`
  flags gaps.
- Every pi worker runs with `--session-control`, so it can be messaged
  over its socket. Its `--name` (the entry id, or `<id>-<label>` for a
  pane added with `cm spawn --into`) is a global alias.
- Watch: the `cm_watch` tool (`start`, `stop`) from the `cm-watch`
  extension.
- Task list: none. `cm reg ls` is the view.
- Subagents: only if a subagent extension is loaded; otherwise do grunt
  work inline.
- Worker skill command: `/skill:claude-manager-worker <mode>`.
- Busy in a capture: `⠧ Working` in the input box's top border. Idle:
  plain border. No placeholder text in the input box.

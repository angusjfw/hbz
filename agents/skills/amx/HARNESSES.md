# Harnesses

What differs between agent harnesses and generic commands for the manager. `amx` handles the
mechanical differences (launch and resume lines, transcript locations,
pane detection); this page covers what you need to decide or know.

A managed session uses one agent harness for all its agent panes, or is
a generic command session. Non-agent command panes can accompany either.
A manager can manage and spawn any supported harness. Use the worker's section
for models, launch and resume; use the manager's section for watch,
task-list and subagent capabilities. Honor an explicit worker harness
choice, otherwise default to the manager's harness.

## Claude Code

- Models: use the installed Claude CLI's model choices, not the
  manager's own model list. Check `claude --help` for supported aliases
  when running from another harness.
  - Mostly invoking a skill, mechanical → smallest.
  - Clear-scope work → mid-size.
  - Real engineering, design, cross-system work, debugging, non-trivial
    review, and the default when unsure → strongest.
- Effort (`--effort`): `low` for mechanical or small, `medium` for
  moderate, `high` up to `xhigh`/`max` for large, ambiguous or hard.
  Effort follows size even on a strong model; a small task at low effort
  doesn't overthink. Blank or thin signal: strongest model, `medium`.
- Resume keeps the model but not effort; `amx` replays the recorded
  effort for the primary worker.
- Watch: `Monitor` on `amx watch --ignore-pane <your pane>`.
- Session hooks: SessionStart and SessionEnd in Claude settings run
  `amx hook --harness claude`.
- Task list: yes.
- Subagents with a model choice: yes.
- Worker skill command: `/amx-worker <mode>`.
- Busy in a capture: a spinner line with `(<elapsed> · ↓ <n> tokens)`.
  Idle: a past-tense line (`✻ Baked for 4s`) above the input box.
  Blocked: a dialog with numbered options and no input box.

## pi

- Models: read `defaultModel`, `defaultThinkingLevel` and
  `enabledModels` from `~/.pi/agent/settings.json` before the first pi
  spawn, regardless of the manager's harness (`pi --list-models <search>`
  expands a glob). If settings are absent, use the CLI defaults or ask.
  The set changes often, so
  there is no fixed table:
  - Most work → the default model.
  - Design, debugging, review → the strongest enabled model.
  - Mechanical, mostly invoking a skill → a small or local model.
  - If the names don't make the ranking obvious, ask the user.
- Effort is pi's thinking level (`off`, `minimal`, `low`, `medium`,
  `high`, `xhigh`, `max`), passed as `--effort` to `amx`. Same sizing as
  Claude.
- Resume restores both model and thinking level.
- pi's process shows only `pi` in `ps`, so a pane's session id can't be
  read from the process. Keep worker lines complete; `amx reconcile`
  flags gaps.
- Messaging is optional. If the session-control extension is installed,
  `amx spawn --agent-arg=--session-control` opts in. Only then can its
  `--name` (the entry id, or `<id>-<label>` for an additional pane) be used
  as a global socket alias. Without it, inspect or interact via tmux.
  Extra arguments are recorded for resume.
- Session hooks: the `amx` extension runs `amx hook --harness pi` on
  session start and on quit, for the interactive session only.
- Watch: use `amx_watch` (`start`, `stop`) if the optional extension is
  loaded. Otherwise reconcile on each manager turn; don't claim idle
  notifications are active.
- Task list: none. `amx ls` is the view.
- Subagents: only if a subagent extension is loaded; otherwise do grunt
  work inline.
- Worker skill command: `/skill:amx-worker <mode>`.
- Busy in a capture: `⠧ Working` in the input box's top border. Idle:
  plain border. No placeholder text in the input box.

## Codex

- Models: use Codex's defaults unless the user chooses a model. Check
  the installed CLI/model picker, not the manager's model list.
- `--effort` maps to native `model_reasoning_effort`. Use values supported
  by the selected model; don't copy another harness's levels blindly.
- Codex chooses its own native ID. amx requests its native session-ID
  terminal title and resolves truncated prefixes against local native
  metadata. An empty thread may not have a persisted ID yet; an unknown
  ID is not a failed process launch, but blocks normal shutdown/resume.
  `amx identify <entry>` records it once available. Never choose a recent
  thread solely because its cwd matches.
- Resume uses `codex resume <full-id>`; no custom hook or skill is required.
- No amx hook or messaging setup is assumed. Native controls and tmux
  interaction remain available. Don't infer busy/idle from another
  harness's spinner conventions.
- As a manager, use the capabilities actually exposed in the session.
  Without idle notifications, reconcile on each turn.

## Generic commands

- Use `amx spawn --id <id> --cwd <dir> --command '<shell command>'`.
  No model, effort, brief or agent ID is assumed. An unsupported agent
  CLI can run this way too, without conversation-resume support.
- Add non-agent panes to any session with
  `amx spawn --into <session> --label <label> --command '<shell command>'`.
- Pause is a registry flag, not OS process suspension. Shutdown snapshots
  and stops the session. Rebuild restarts the recorded command, including
  its side effects; it does not restore application state.
- Hooks, auto-registration, agent busy state and messaging are optional
  capabilities, not requirements for core lifecycle operations.

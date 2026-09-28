# amx reference

Formats and states behind `amx` and its skills. Change files only through
`amx`; this page is for reading them. Print it with `amx reference`.

## Paths

Under `~/.local/state/amx/`:

- `sessions.md`: the registry.
- `snapshots/<id>.txt`: pane captures from shutdown or wrap.
- `resume/<id>.md`: resume_state from shutdown.
- `briefs/<name>.md`: spawn briefs.
- `.last-writer`: pane id of the last `amx reg` writer.
- `session-log.jsonl`: one line per agent session start and end (from the
  hooks) and per removed entry (from `amx wrap` and `amx reg rm`, with the
  entry's fields). The record of everything that ran.
- `hook-errors.log`: anything that went wrong inside `amx hook`.

## Registry

```markdown
# Sessions

manager: amx:1.0 harness=claude

## eng-1234-payment-bug
harness: claude
ticket: ENG-1234
tmux_session: eng-1234-payment-bug
cwd: ~/code/repo/_wt/eng-1234
worktree: ~/code/repo/_wt/eng-1234
branch: fix/eng-1234
model: opus
effort: high
started: 2026-04-29 14:00
last_touched: 2026-04-29 16:20
resumed_session_id: 7f1c9e02-4b6a-4d51-9f83-2ac0be7d5511
worker: c40b8d17-9e22-4a76-bb31-6e5f0d92a418 cwd=~/code/repo/_wt/eng-1240 label=api
notes: ~/code/journal/2026-04-29-eng-1234.md
```

Header lines (before the first `## `):

- `manager: <tmux address> harness=<h>`: one per running manager.

Entry fields (all optional; unknown fields and prose are kept):

- `harness`: `claude` or `pi`. Absent means `claude`.
- `auto`: `true` on an entry the session hooks created. It goes away on
  `amx track`, pause or shutdown. An auto entry is removed when its
  primary agent exits.
- `tmux_session`: the tmux session, normally equal to the entry id.
  Present iff the session is alive.
- `cwd`, `worktree`, `branch`, `ticket`.
- `model`, `effort`: the spawn choice. For pi, `effort` is the thinking
  level.
- `started`, `last_touched`, `shutdown`, `paused`: timestamps. Always set
  from the clock (`k=now` / `k=today`), never typed.
- `resumed_session_id`: the primary worker's session id (first window,
  pane 0). Set at spawn, never truncated.
- `worker`: repeatable, one per agent pane beyond the primary:
  `<session-id> cwd=<path> [label=<name>]`. No window or pane position;
  those renumber.
- `snapshot`, `resume_state`: paths written at shutdown or wrap.
- `resume_target`: expected resume date, free-form.
- `wrap_requested`: only on entries from before workers wrapped
  themselves; the wrap is unfinished.
- `notes`: text, or a path to a notes file.

## States

Derived from which fields are present:

| State | Fields |
|---|---|
| active | `tmux_session` |
| paused | `tmux_session` + `paused` (tmux option `@amx_paused` set too) |
| shutdown | no `tmux_session`; `shutdown` + `resume_state` + `resumed_session_id` |
| wrap requested (old) | no `tmux_session`; `wrap_requested: true` |

Any state can also carry `auto`. `amx ls` prints the state per entry
(`-a` includes auto ones). A wrapped entry is gone from the registry and
kept in the session log; `amx log --removed [id]` shows how it went and
its notes.

## Who writes what

- The session hooks create auto entries, record each agent's session id
  as it starts, and remove auto entries (and worker lines) as agents
  exit. They never touch a shared session: one tmux auto-named (a bare
  number) or one a manager runs in.
- A worker writes only its own entry: `last_touched`, `notes`, `ticket`,
  `branch`, `paused`, `worker`, the shutdown fields, and its own wrap.
- The manager owns the header, spawn and cold resume.
- Coordination skills add and drop `worker:` lines on the entry of the
  session they run in (`amx spawn --into`, `amx reg worker drop`).

## resume_state

```markdown
# Resume state: eng-1234

shutdown: 2026-05-22 18:00
harness: claude

## window 1: eng-1234
layout: 5fe4,200x50,0,0,0

### pane 0
cwd: /home/me/code/repo/_wt/eng-1234
command: claude --effort high --resume 7f1c9e02-…
harness: claude
session_id: 7f1c9e02-…

### pane 1
cwd: /home/me/code/repo
command: yarn dev
```

- One `## window <n>: <name>` block per window, in order, with `layout:`.
- One `### pane <n>` block per pane: `cwd:`, `command:` (empty for an idle
  shell), and on agent panes `harness:`, `session_id:`, `label:`.
- Old files use `claude_session_id:`; it reads as `session_id:` with
  harness `claude`.
- Any other `## ` section is prose for the reader.
- `amx rebuild` builds agent panes' commands from `session_id` and
  `harness` (so pi panes always get `--session-control`), and replays
  other panes' `command:` as written.

## Snapshots

`amx snapshot` writes `--- window <w> pane <p> (<kind> <session-id>) ---`
then the capture. Claude panes get a `--- input box: <class> ---` line:
`ghost` is Claude's own placeholder guess, `draft` is text the user typed
and didn't send. Never read box text as a message from the user.

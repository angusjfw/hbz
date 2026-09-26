# claude-manager reference

Formats and states shared by the manager and worker skills. Change files
only through `cm`; this page is for reading them.

## Paths

Under `~/.local/state/claude-manager/`:

- `sessions.md`: the registry.
- `snapshots/<id>.txt`: pane captures from shutdown or wrap.
- `resume/<id>.md`: resume_state from shutdown.
- `briefs/<name>.md`: spawn briefs.
- `.last-writer`: pane id of the last `cm reg` writer.

## Registry

```markdown
# Sessions

manager: claude-manager:1.0 harness=claude

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
- `wrap_requested`: `true` when a worker has wrapped itself and the
  manager hasn't written the journal yet.
- `notes`: text, or a path to a notes file.

## States

Derived from which fields are present:

| State | Fields |
|---|---|
| active | `tmux_session` |
| paused | `tmux_session` + `paused` (tmux option `@cm_paused` set too) |
| shutdown | no `tmux_session`; `shutdown` + `resume_state` + `resumed_session_id` |
| wrap requested | no `tmux_session`; `wrap_requested: true` |

`cm reg ls` prints the state per entry.

## Who writes what

- A worker writes only its own entry: `last_touched`, `notes`, `ticket`,
  `branch`, `paused`, `worker`, and the shutdown/wrap fields.
- The manager owns the header, spawn, cold resume, entry removal and the
  wrap journal entry.
- Coordination skills add and drop `worker:` lines on the entry of the
  session they run in (`cm spawn --into`, `cm reg worker drop`).

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
- `cm rebuild` builds agent panes' commands from `session_id` and
  `harness` (so pi panes always get `--session-control`), and replays
  other panes' `command:` as written.

## Snapshots

`cm snapshot` writes `--- window <w> pane <p> (<kind> <session-id>) ---`
then the capture. Claude panes get a `--- input box: <class> ---` line:
`ghost` is Claude's own placeholder guess, `draft` is text the user typed
and didn't send. Never read box text as a message from the user.

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

- `manager: <tmux address> harness=<h>`: one per running manager. The
  harness describes its runtime, not which workers it may manage.

Entry fields (all optional; unknown fields and prose are kept):

- `harness`: `claude`, `pi`, `codex` or `command`. Absent means `claude`.
- `command`: the shell command for a generic session. Restarted as written
  by `/bin/sh`; no agent session ID or conversation-resume semantics.
- `auto`: `true` on an entry the session hooks created. It goes away on
  `amx track`, pause or shutdown. An auto entry is removed when its
  primary agent exits.
- `tmux_session`: the tmux session, normally equal to the entry id.
  Present iff the session is alive.
- `cwd`, `worktree`, `branch`, `ticket`.
- `model`, `effort`: the spawn choice. For pi, `effort` is the thinking
  level.
- `agent_args`: JSON array of explicit extra CLI arguments for the primary
  agent, passed with repeatable `spawn --agent-arg=ARG`. For example,
  `--agent-arg=--session-control` opts into pi's separately installed
  messaging extension; it is not part of the default launch. Commands and
  arguments are stored as plaintext; do not put credentials in them.
- `started`, `last_touched`, `shutdown`, `paused`: timestamps. Always set
  from the clock (`k=now` / `k=today`), never typed.
- `resumed_session_id`: the primary agent's session id (first window,
  pane 0), never truncated. Assigned at spawn for Claude/pi; discovered
  from native identity for Codex. Absent for generic commands and while
  an agent's native identity is unknown.
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
| shutdown | no `tmux_session`; `shutdown` + `resume_state`; agent IDs recorded when known |
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
  shell), and on agent panes `harness:`, `session_id:`, `label:`. Managed
  command panes retain their launch cwd even if the process changes directory.
- Old files use `claude_session_id:`; it reads as `session_id:` with
  harness `claude`.
- Any other `## ` section is prose for the reader.
- `amx rebuild` builds agent panes' commands from `session_id`, `harness`
  and optional `agent_args` (a JSON array), and replays other panes'
  `command:` as written. Explicit extra arguments survive shutdown/resume.
  A declared agent command without `session_id` is rejected, not silently
  treated as a generic restart. Leave its command empty for a shell if
  the native ID cannot be recovered; fresh agent starts are explicit.

## Harness capabilities

| Harness | Launch identity | Cold resume |
| --- | --- | --- |
| `claude` | Native `--session-id` | Native `--resume` |
| `pi` | Native `--session-id` | Exact saved project session; missing transcript is an error |
| `codex` | Native terminal title, resolved to a full ID | Native `codex resume <id>` |
| `command` | None | Restart the recorded shell command |

Core management requires Python 3 and tmux, not agent hooks, skills or
extensions. Optional hooks add auto-registration and session-switch
tracking; optional status integrations add busy state and notifications.
CLI-specific flags, identity and resume behavior live in the built-in
harness adapter functions. Other CLIs can use `command` without an adapter:

```bash
amx spawn --id server --cwd /path/to/project --command 'make dev'
amx spawn --into task --label logs --command 'tail -f app.log'
amx spawn --harness codex --id review --cwd /path/to/project
```

Agent panes in a session use one harness. Generic command panes can be
added to any session; their labels and exact commands survive shutdown.
Commands run with the launch environment; amx does not snapshot secrets
or arbitrary environment variables. Restarting a command may repeat its
side effects. Review the recorded command before rebuilding.

### Codex identity

amx sets Codex's native `tui.terminal_title=["session-id"]` for its own
launches and resumes. This changes the pane title, not saved Codex config.
A full UUID is accepted directly. A truncated native UUID prefix must
resolve to exactly one full ID in Codex's local thread index or rollout
metadata for the same cwd (or match a recorded full resume ID). amx never
chooses the newest session or copies an ID out of conversation text.

An empty thread may not be persisted yet. Startup still reports the
command as running, but prints `session_id=unknown`. Once Codex has saved
the thread, `amx identify <id>` records its native ID; shutdown also
records it automatically. Unknown IDs block normal shutdown. A native ID
is not proof of saved conversation history: an empty renamed thread can
have an index entry but no history to resume. If a CLI
version, remote server or storage override prevents native discovery,
bind the full ID explicitly with
`amx identify <entry> --pane <pane-id> --session-id <full-native-id>`.
This updates the pane-bound identity and registry together, including for
additional workers. Do not infer it from another session in the same directory.

Discovery reads `CODEX_HOME` (default `~/.codex`), `CODEX_SQLITE_HOME` when
set, local `state_*.sqlite` thread indexes and JSONL rollout metadata.
`amx transcripts codex <cwd>` searches local JSONL rollouts; other native
storage formats may require Codex's own session browser. Native IDs and
stored transcripts are separate capabilities.

## Startup observation

`amx spawn` runs the command directly under tmux with `remain-on-exit`.
It observes the same live pane process for two seconds before reporting
`startup=running (readiness not verified)`. This is not proof that login,
model loading or agent initialization has finished. Failures after that
window remain possible; `amx reconcile` reports retained exited panes.

An early exit (including status 0), missing pane or observation timeout
returns exit 3. The registry entry and any surviving pane are retained for
inspection. This applies to both new sessions and `spawn --into`.
Rebuild failure leaves the entry shut down and removes the partial tmux
session; its output is captured in `snapshots/<id>-rebuild-failed.txt`
when possible, without overwriting the original recovery data.

Hooks and the agent-status store are optional, not startup acknowledgements.
Managed panes carry harness, session ID and extra arguments in tmux pane
options, allowing snapshot and resume without hooks. Without hooks, an
agent's in-process session switch must be recorded explicitly before
shutdown; the launch ID alone cannot follow a new conversation. Use
`amx identify <entry> --pane <pane-id> --session-id <full-native-id>` to
correct a pane's identity, rather than merely adding a second worker line.
When installed, session-start hooks update the pane-bound ID on switches.

## Snapshots

`amx snapshot` writes `--- window <w> pane <p> (<kind> <session-id>) ---`
then the capture. Claude panes get a `--- input box: <class> ---` line:
`ghost` is Claude's own placeholder guess, `draft` is text the user typed
and didn't send. Never read box text as a message from the user.

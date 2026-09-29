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
worker: c40b8d17-9e22-4a76-bb31-6e5f0d92a418 harness=pi cwd=~/code/repo/_wt/eng-1240 label=api
notes: ~/code/journal/2026-04-29-eng-1234.md
```

Header lines (before the first `## `):

- `manager: <tmux address> harness=<h>`: one per running manager. The
  harness describes its runtime, not which workers it may manage.

Entry fields (all optional; unknown fields and prose are kept):

- `harness`: the primary/default harness: `claude`, `pi`, `codex` or
  `command`. Absent means `claude`. It does not restrict other panes.
- `command`: the shell command for a generic session. Restarted as written
  by `/bin/sh`; no agent session ID or conversation-resume semantics.
- `auto`: `true` on an entry the session hooks created. It goes away on
  `amx track`, pause, shutdown or attachment of another managed pane.
  An auto entry is removed when its primary exits only if no other panes
  or worker records remain.
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
- `resumed_session_id`: the primary agent's session id, never truncated.
  The primary starts in the first window's pane 0; managed panes retain
  their roles across renumbering and rebuilds. Assigned at spawn for
  Claude/pi; discovered from native identity for Codex. Absent for generic commands and while
  an agent's native identity is unknown.
- `worker`: repeatable, one per agent pane beyond the primary:
  `<session-id> harness=<h> cwd=<path> [label=<name>] [model=<m>]
  [effort=<e>] [agent_args=<JSON>]`. Values use shell quoting when needed.
  Identity is `(harness, session-id)`; the same native ID in another
  harness is distinct. Legacy lines without `harness` inherit the entry's
  harness and are upgraded when refreshed, not by a bulk migration.
  Labels are unique within the entry. No window or pane position; those
  renumber. Command panes have no worker ID; their commands and labels
  live in pane metadata and resume_state.
  `amx reg worker add <entry> <sid> harness=<h> ...` upserts that identity.
  `amx reg worker drop <entry> <sid> --harness <h>` removes only that
  harness's worker; omitting the harness is rejected if ambiguous.
  Unresolved panes and labelled recovery shells can retain last-known
  records. These preserve recovery data, not proof of a current ID:
  consult `amx panes` or `amx resolve` before resuming or messaging.
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
(`-a` includes auto ones). Its columns are id, primary harness, state,
busy, ticket, cwd and member harnesses. `--harness` matches any recorded
member, including pane metadata and saved resume_state when available.
A wrapped entry is gone from the registry and
kept in the session log; `amx log --removed [id]` shows how it went and
its notes.

## Who writes what

- A primary session-start hook can create an auto entry; a secondary
  event alone never invents its parent's harness. Hooks record each
  agent's session id as it starts, and remove auto entries (and worker lines) as agents
  exit. They never touch a shared session: one tmux auto-named (a bare
  number) or one a manager runs in.
- The primary worker owns the container's lifecycle. Secondary workers
  share its entry; finishing one does not finish the container. Pause,
  shutdown and wrap affect the whole entry, regardless of harness.
  Shutdown/wrap from a secondary pane require `--whole-session`, only
  after the user explicitly agrees to affect the entire container.
  A manager targeting another session is not subject to that caller guard.
- The manager owns the header, spawn and cold resume.
- Coordination skills add and drop `worker:` lines on the entry of the
  session they run in (`amx spawn --into`, `amx reg worker drop`).

## Pane identity and routing

`amx resolve <entry-or-worker-name-or-%pane-id> --json` returns `name`,
`pane_id`, `tmux_session`, `address`, `entry`, `entry_harness`, `harness`,
`session_id`, `cwd`, `primary`, `label`, `model`, `effort`, `agent_args`
(an array), `dead`, `harness_conflict` and `state` (the pane's
agent-status state, or null). It does not send input; `amx msg` does.

The primary's name is the entry id; a labelled worker's is
`<entry>-<label>`. Unlabelled or unregistered panes use their explicit
`%pane-id`. Concatenated names can collide across entries; resolution
rejects ambiguity and lists pane IDs rather than choosing a target.
These are amx addresses, not promises about native messaging aliases.

`amx whoami --json` exposes the same pane-local identity, retaining
`pane` and `session` as aliases for `pane_id` and `tmux_session`.
`harness` describes this pane; `entry_harness` is the primary/default.
Command/shell panes report `harness=command`, not a chat capability.
Socket availability, drafts and readiness must be checked at delivery.

Managed panes carry `@amx_harness`, `@amx_session_id`, `@amx_label`,
`@amx_primary`, `@amx_model`, `@amx_effort`, `@amx_agent_args`,
`@amx_command` and `@amx_launch_cwd`. Names are derived from the current
registry, so a renamed entry does not leave a stale cached amx address.

## Messaging

Same-harness sessions use their native tools: `SendMessage` between
Claude Code sessions, `send_to_session` between pi sessions running the
session-control extension. `amx msg` is for everything else. It needs
no manager and no registry entry on either side.

```
amx msg TARGET (TEXT | --file F) [--via auto|socket|pane] [--steer]
        [--wait SECS] [--force]
```

TARGET is anything `amx resolve` accepts. The route:

- pi target whose `~/.pi/session-control/<session_id>.sock` answers:
  the socket (`follow_up`; `--steer` interrupts the current turn).
- Any other chat agent (Claude, Codex, pi without a socket): typed into
  the pane. A bracketed paste, Escape if the pane shows vim INSERT mode,
  then Enter.
- `--via socket` or `--via pane` forces a route; a forced route that
  can't work is refused, never swapped.

Typed messages start with `[from <name> (<harness>, <%pane>); reply: amx
msg <name> "..."]`. Over the socket from pi, a `<sender_info>` tag lets
pi reply natively.

Success prints `delivered via=<socket|pane> target=<name> pane=<%id>
verified=<yes|no>` and exits 0. `verified=no` means the box didn't read
empty afterwards (always, for Codex); look at the pane. A refusal prints
`refused <reason>: <detail>` to stderr and exits 1; nothing was sent.
Other errors exit 2.

| reason | meaning | what to do |
| --- | --- | --- |
| `native` | same harness, native tool reaches it | use the named tool; `--via pane` if it holds the message |
| `busy` | the target is working | retry later or `--wait SECS`; `--force` only if the pane shows it idle (an interrupt leaves `working` stale) |
| `needs-input` | a permission prompt or question is up | tell the user; never forced |
| `draft` | unsent text in the target's box | ask the user; never forced |
| `box-unknown` | no box on screen, the pane is in tmux copy mode, or no reader (Codex) | look at the pane; `--force` only when there's no reader and nobody is typing there |
| `state-unknown` | no busy state (no status integration) | look at the pane; `--force` if it's idle |
| `no-socket` | `--via socket` without a live socket | drop `--via`, or start pi with `--session-control` |
| `command` | not a chat agent | use tmux-interaction deliberately |
| `dead`, `unresolved`, `ambiguous`, `self` | no usable target | check the name; use a `%pane` ID |

Never `--force` into a pane the user may be typing in. Senders to one pane
take turns: checks and typing run under a per-pane lock in the state dir.

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
- One `### pane <n>` block per pane: `cwd:`, `primary:` (`true`/`false`),
  `command:` (empty for an idle shell), and on agent panes `harness:`,
  `session_id:`, `label:` plus recorded `model:`, `effort:` and `agent_args:`.
  Legacy files without `primary:` use the first pane of the first window.
  Managed command panes retain their launch cwd even if the process
  changes directory.
- Old files use `claude_session_id:`; it reads as `session_id:` with
  harness `claude`.
- Any other `## ` section is prose for the reader.
- `amx rebuild` builds agent panes' commands from `session_id`, `harness`
  and per-pane effort/extra arguments, and replays other panes' `command:`
  as written. Native resume restores models; Claude/Codex effort and
  explicit extra arguments are replayed for secondary agents too.
  A declared agent command without `session_id` is rejected, not silently
  treated as a generic restart. Leave its command empty for a shell if
  the native ID cannot be recovered; fresh agent starts are explicit.
  Its label and role survive as a shell, not as a chat target.
- Without a resume_state, the one-pane fallback is allowed only when no
  workers are recorded. If workers exist, recover their per-pane state
  first; rebuild refuses to discard their recovery pointers.

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

A session can contain any mix of supported agents and generic commands.
Use `--into <session> --harness <h> --label <label>` for an agent split;
add `--window <name>` for a new window in that same session. Without an
explicit harness, `--into` uses the entry's default. A requested split
must belong to the requested session. Never replace a requested pane or
window with a separate session or unmanaged launch without agreement.

Generic panes' labels and exact commands survive shutdown.
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
When installed, session-start hooks update each pane's ID, harness and
role on switches without changing a secondary's parent identity.
Mixed membership is valid. `reconcile` reports `harness-conflict` only
when pane metadata disagrees with an observed agent process; inspect and
explicitly correct the pane identity before saving it.

## Snapshots

`amx snapshot` writes `--- window <w> pane <p> (<kind> <session-id>) ---`
then the capture. Claude panes get a `--- input box: <class> ---` line:
`ghost` is Claude's own placeholder guess, `draft` is text the user typed
and didn't send. Never read box text as a message from the user.

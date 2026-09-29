# Crash recovery

The tmux server died without a shutdown (machine restart, crash, an
accidental kill). Every live entry lost its session at once and none has
a fresh resume_state. Nothing was written on the way out, so the work is
reading the evidence that survived and reconciling it.

## Rule zero: read everything before creating anything

Rebuilding one session destroys evidence about the others, in particular
the only record of agent panes that were never registered (step 1).
Gather all three sources, reconcile, show the user, and only then
rebuild.

## 1. Copy the agent-status store aside, first

```bash
cp -R ~/.local/state/agent-status \
  ~/.local/state/amx/agent-status.crash-$(date +%Y%m%dT%H%M%S)
```

It maps every agent session id (Claude and pi) to its pane, per tmux
session, so it can name conversations nothing else knows about. It wipes
itself during recovery: pane ids don't survive a server restart, and the
first event in a recreated session drops every other id recorded for it.

## 2. Read the background save

tmux-resurrect under tmux-continuum saves the whole server to
`~/.local/state/tmux/resurrect/last` every few minutes. Tab-separated:

| Record | Fields |
| --- | --- |
| `pane` | session, window index, window active, window flags, pane index, pane title, pane cwd, pane active, pane command, full command |
| `window` | session, window index, window name, window active, window flags, window layout, automatic-rename |
| `state` | client session, client last session |

Pane contents are in `pane_contents.tar.gz` in the same directory.

Check its mtime against the time of death. It goes stale silently when
no client was attached (autosave runs from the status line), or when a
second tmux server was running when the config loaded (continuum then
doesn't save at all). If it's stale or missing, recovery still works
from the registry and step 4's routes, without layouts or non-agent
commands. Say so.

Restore isn't wired into tmux on purpose: sessions shut down on purpose
must stay down, and the registry is reconciled in the same pass.

## 3. Read the registry

`amx reg show`. It is the authority on what *should* exist: which
entries were shut down (leave them), paused, or wrap-requested. The save
file only says what was running.

## 4. Reconcile, show, then rebuild

Merge the three sources into one list and show it to the user before
acting. They confirm the set once, not session by session.

Per agent pane, take the session id from the first route that has it:

1. The entry's `resumed_session_id` or a `worker:` line.
2. Claude: the saved full command's `--session-id` / `--resume`.
   pi: the saved full command is just `pi`. Use the pane title instead,
   `π - <name> - <dir>`: `<name>` is the entry id for the primary, or
   `<id>-<label>` for a worker, which a `worker:` line's `label=` maps to
   an id. Failing that, `amx transcripts pi <cwd> --name <name>`.
   Codex: a saved `codex resume <id>` command carries the native ID.
   Otherwise its saved session-ID title may be a truncated prefix; match
   it uniquely against local Codex metadata/transcripts, never by cwd or
   mtime alone. `amx transcripts codex <cwd> --grep <prefix>` can locate
   JSONL rollouts; use Codex's own session browser for other storage.
3. The session log: the last `start` line for that tmux session and pane
   (`grep '"pane": "%<id>"' ~/.local/state/amx/session-log.jsonl`, or by
   tmux session and cwd, since pane ids don't survive the restart).
4. The copied agent-status store.
5. A transcript content hunt: `amx transcripts <harness> <cwd> --grep …`.

Then per entry, write a resume_state from the save file (format in
`REFERENCE.md`), set it with
`amx reg set <id> resume_state=<path> shutdown=now` and
`amx reg unset <id> tmux_session`, and run `amx rebuild <id>`.

An unknown agent ID is not permission to create a new conversation.
Leave its recovery command empty so the pane returns as a shell; start and
register a fresh agent only with the user's agreement. Say so per pane,
rather than calling it resumed. Rebuild rejects a declared agent command
without an ID. Generic command sessions have no conversation ID: review
their recorded command and restart it, with its possible side effects.

## 5. Record what happened

Per entry: `amx reg set <id> notes="…"` saying it was rebuilt after a
crash and which ids came back by which route. Re-add `worker:` lines for
every non-primary pane brought back (`amx reconcile` lists them).

## An agent pane with no registry entry

A coordination skill's worker that was never recorded. The save file's
pane title and cwd usually identify it; its first prompt says more. If
its parent can't be established, give it its own entry rather than
guessing a parent: a wrong parent puts it under a lifecycle that doesn't
own it.

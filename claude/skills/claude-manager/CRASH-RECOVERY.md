# Crash recovery

The tmux server died without a shutdown — machine restart, crash, an
accidental kill. Every live entry lost its container at once and none of
them has a `resume_state`.

Distinct from Cold resume, which rebuilds one session from a record
written deliberately. Here nothing was written on the way out, so the
work is reading evidence that happens to survive and reconciling it.

Loaded on demand from `SKILL.md` § Crash recovery — the case is rare and
the mechanics are long, so they don't sit in the manager's context by
default. The one thing the manager needs *before* loading this is the
first rule below: don't create anything yet.

## Rule zero: read everything before creating anything

Rebuilding a session destroys evidence about the others. Specifically, it
destroys the only record of Claude panes that were never registered — see
step 1. So the order is: gather all three sources, reconcile, surface,
and only then rebuild.

## 1. Copy the agent-status store aside, first

```bash
cp -R ~/.local/state/agent-status \
  ~/.local/state/claude-manager/agent-status.crash-$(date +%Y%m%dT%H%M%S)
```

The store maps every Claude session id to its pane, per tmux session, so
it can name conversations no other source knows about — including a
coordinator's unregistered sub-workers, which is exactly the gap that
hurts here.

It is a live status store, not a record, and it self-destructs at
recovery time. Each hook event drops any Claude whose recorded `pane_id`
no longer runs one; pane ids do not survive a server restart, so the
first hook event in a recreated session wipes every *other* id from that
session's entry. The pre-crash inventory is readable up until the first
session is rebuilt and not after.

Copy it before touching anything, not when it is needed.

## 2. Read the background save

The dotfiles repo's tmux config runs tmux-resurrect under
tmux-continuum, writing the whole server to
`~/.local/state/tmux/resurrect/last` every few minutes: sessions,
windows, layouts, pane cwds, pane titles, and each pane's full command
line — including its `--session-id` / `--resume` argument, which is what
makes a pane's conversation identifiable without a transcript hunt.

Tab-separated, three record types:

| Record | Fields |
| --- | --- |
| `pane` | session, window index, window active, window flags, pane index, pane title, pane cwd, pane active, pane command, full command |
| `window` | session, window index, window name, window active, window flags, window layout, automatic-rename |
| `state` | client session, client last session |

Pane contents for each pane are in `pane_contents.tar.gz` in the same
directory.

**Check its mtime against the time of death before trusting it.** Two
ways it goes stale, both silent:

- Autosave runs from a `status-right` hook, so it only fires while a
  client is attached. A long detached stretch saves nothing.
- continuum does not install the hook at all if a second tmux server was
  running when the config loaded, to stop two servers overwriting each
  other's save file. A stray scratch server therefore disables the
  safety net with no warning.

If the save is stale or missing, recovery still works — it just falls
back to the registry plus the id routes in step 4, and loses layout and
non-Claude pane commands. Say so rather than presenting a thin rebuild
as complete.

Restore is deliberately not wired to tmux (no auto-restore, and
`prefix+C-r` unbound) because rebuilding is the manager's job, not a
keystroke's: the registry has to be reconciled in the same pass, and
sessions that were shut down on purpose must stay down.

## 3. Read the registry

The authority on what *should* exist and on lifecycle — which entries
were deliberately `shutdown` (leave them down), which were `paused`,
which carry `wrap_requested`. The save file only says what was running,
which is not the same question.

## 4. Reconcile, surface, then rebuild

Reconcile the three sources into one list and **surface it before
acting.** This is a mass operation, so the user confirms the set once,
not each session.

Then per entry, cold-resume mechanics apply (windows, splits, layout,
sends — see `SKILL.md` § Cold resume), with the save file standing in for
`resume_state`.

Per Claude pane, take the id from the first route that has it:

1. The entry's `resumed_session_id`, or one of its `worker:` lines.
2. The saved `full command` — `--session-id` / `--resume` argument.
3. The copied agent-status store.
4. A transcript content hunt (`SKILL.md` § Untracked cold resume).

A pane whose id can't be established comes back as a **fresh** Claude.
Say that explicitly per pane; a fresh Claude in a rebuilt session looks
resumed, and the user will otherwise assume the context is there.

## 5. Record what happened

Per entry, in `notes`: that it was rebuilt after a crash, and which ids
came back by which route. A recovery that isn't recorded gets re-litigated
the next time someone reads the entry.

Re-add a `worker:` line for every non-primary pane brought back.

## A Claude pane with no registry entry

A coordinator's sub-worker that was never recorded. The save file's pane
title and cwd usually identify it, and its own first prompt says more.

If its parent can't be established, give it its own registry entry rather
than guessing a parent — a wrong parent is worse than an orphan, since it
puts the session under a lifecycle that doesn't own it.

Treat it as a failure of the `worker:` discipline (`SKILL.md` § Registry),
not of recovery. It is the one loss this flow cannot fully undo: the
conversation comes back, its place in the team does not.

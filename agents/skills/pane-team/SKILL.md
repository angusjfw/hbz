---
name: pane-team
description: >-
  Work alongside other agents running in visible tmux panes: spawn a worker
  into a pane, brief it, and message it (and let it message you or other
  workers). Use for multi-part or long-running work the user wants to watch and
  steer, or when asked to coordinate, spawn a worker, run tasks in parallel,
  split work across panes, or get another agent's eyes on something. For a
  short self-contained lookup or task, use a subagent instead.
---

# pane-team

Workers are full agent sessions in panes of the user's tmux server, visible
and steerable by the user. A pane is the unit; a window just holds panes.
Workers run the same agent CLI you do (`claude` or `pi`).

## Pick a shape

These are starting points, not roles. Mix them and change as the work moves.

- **Fan-out**: workers take independent parts; you integrate and check.
- **Pair**: one worker builds, another reviews; they talk to each other directly.
- **Watch**: the user drives a worker pane; you follow along and advise.
- **Relay**: each worker's output feeds the next.

Anyone may message anyone. Say who a worker should report to in its brief.

## Spawn a worker

Every worker gets a short unique name (e.g. `api-tests`). It is how everyone
addresses that worker. Write the brief to a file.

**Inside a managed session** (`amx whoami` prints a non-empty `entry=`), spawn
through `amx` so the worker is registered and comes back when the session is
shut down and resumed:

```bash
amx spawn --into "$session" --label "$name" --cwd "$cwd" --brief-file "$brief_file" \
  [--model <model>] [--window <name>]
```

It splits beside the active pane (or opens a window with `--window`) and
prints `pane=`, `name=` and `session_id=`. The printed name
(`<entry>-<label>`) is the worker's address; keep the `session_id` for
Finish.

**Otherwise**, start it yourself:

```bash
tmux split-window -d -P -F '#{pane_id}' -c "$cwd" \
  "<agent> --name $name \"\$(cat $brief_file)\""
```

where `<agent>` is `claude`, or `pi --session-control` (pi needs it to be
messageable).

- Split beside your own pane, or use `new-window -d` when the window is full.
  Check the layout first (`tmux list-panes -F '#{pane_id} #{pane_current_command}'`).
- Add `--model <model>` for a cheaper or stronger worker. Default: yours.
- Tell the user the name and pane id.

## Brief

Include: the user's own words for the goal, what done looks like, where the
work happens (cwd, branch), who to report to (your own name), and anything the
worker must not touch.

## Talk

- Message workers by name with your harness's cross-session messaging tool
  (Claude Code: `SendMessage`, with `ListAgents` to see names; pi:
  `send_to_session`, with `list_sessions`). Prefer it over typing into panes.
- Ask workers to message you when they finish or get stuck. Don't wait on a
  turn ending: a turn can end long before the task does.
- Claude Code holds messages to a session in a different permission mode for
  the user's approval, so start workers in the mode you run in.
- If no messaging tool reaches the worker, fall back to `tmux send-keys` to
  type into its pane and `tmux capture-pane -p` to read it.

## Keep track

For work longer than a few exchanges, keep a short status file (who is doing
what, what's done, what's blocked) in the working directory or `docs/plans/`.
Messages scroll away; the file is what you and the user come back to.

## Finish

When a worker is done and the user has seen its result, close its pane
(`tmux kill-pane -t <id>`), or leave it open if the user may want to look.
In a managed session, unregister it first with
`amx reg worker drop <entry> <session_id>`; a worker left open stays
registered.

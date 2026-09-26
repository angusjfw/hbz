---
name: pane-team
description: >-
  Work alongside other pi agents running in visible tmux panes: spawn a worker
  into a pane, brief it, and message it (and let it message you or other
  workers) over pi's session-control sockets. Use for multi-part or long-running
  work the user wants to watch and steer, or when asked to coordinate, spawn a
  worker, split work across panes, or get another agent's eyes on something.
  For a short self-contained lookup or task, use the subagent tool instead.
---

# pane-team

Workers are full pi sessions in panes of the user's tmux server, visible and
steerable by the user. A pane is the unit; a window just holds panes.

## Pick a shape

These are starting points, not roles. Mix them and change as the work moves.

- **Fan-out**: workers take independent parts; you integrate and check.
- **Pair**: one worker builds, another reviews; they talk to each other directly.
- **Watch**: the user drives a worker pane; you follow along and advise.
- **Relay**: each worker's output feeds the next.

Anyone may message anyone. Say who a worker should report to in its brief.

## Spawn a worker

Write the brief to a file, then:

```bash
tmux split-window -d -P -F '#{pane_id}' -c "$cwd" \
  "pi --session-control --name $name \"\$(cat $brief_file)\""
```

- Split beside your own pane, or use `new-window -d` when the window is full.
  Check the layout first (`tmux list-panes -F '#{pane_id} #{pane_current_command}'`).
- `$name` is short and unique (e.g. `api-tests`). It is how everyone addresses
  that worker. Tell the user the name and pane id.
- Add `--model <provider/id>` for a cheaper or stronger worker. Default: yours.

**Inside a managed session**, spawn through `cm` instead so the worker is
registered and comes back when the session is shut down and resumed. Check
with `cm whoami`: a non-empty `entry=` means managed.

```bash
cm spawn --into "$session" --label "$name" --cwd "$cwd" --brief-file "$brief_file" \
  [--model <provider/id>] [--window <name>]
```

It splits beside the active pane (or opens a window with `--window`) and
prints `pane=`, `name=` and `session_id=`. Use the printed `name`
(`<entry>-<label>`) to address the worker, and keep the `session_id` for
Finish.

## Brief

Include: the user's own words for the goal, what done looks like, where the
work happens (cwd, branch), who to report to (your name: `$PI_SESSION_ID` or
your `--name`), and anything the worker must not touch.

## Talk

- Use the `send_to_session` tool with `sessionName`. `mode: follow_up` waits
  for the worker to finish its current step; `steer` interrupts it.
- Don't use `wait_until: turn_end` to wait for a whole task. A turn ends after
  the worker's first tool step, not when the task is done. Ask workers to
  message you back when they finish or get stuck.
- `list_sessions` shows who is running.
- You can only use these tools if you were started with `--session-control`.
  Without it, fall back to `tmux send-keys` and `tmux capture-pane`.
- From a shell or script:
  `pi -p --session-control --control-session <name> --send-session-message "..." --send-session-wait turn_end`

## Keep track

For work longer than a few exchanges, keep a short status file (who is doing
what, what's done, what's blocked) in the working directory or `docs/plans/`.
Messages scroll away; the file is what you and the user come back to.

## Finish

When a worker is done and the user has seen its result, close its pane
(`tmux kill-pane -t <id>`), or leave it open if the user may want to look.
In a managed session, unregister it first with
`cm reg worker drop <entry> <session_id>`; a worker left open stays
registered.

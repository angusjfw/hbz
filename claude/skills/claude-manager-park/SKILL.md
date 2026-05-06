---
name: claude-manager-park
description: Park a Claude session from inside the worker pane — moves the worker's tmux window into a standalone tmux session so it stays running but drops out of the manager's window list. Reversible. Worker self-serves under the registry lock; manager observes via its watch. Use when in a worker pane and you want to park the session without switching to the manager. Optional argument is the target tmux session name; defaults to the worker's window name.
---

# claude-manager-park

`/claude-manager-park [<target-tmux-session-name>]`

Park = move this worker's tmux window out of the manager's window list
into a standalone tmux session. Worker keeps running. Reversible — the
manager can move it back later when asked.

This is a thin wrapper. The shared lifecycle flow lives at
`../claude-manager-end/FLOW.md`. Read it and run with `mode=park`.

The worker handles park itself: captures its tmux address, acquires the
registry lock, performs the `tmux move-window`, rewrites its registry
entry (`tmux_window` → `tmux_session`), releases the lock, and tells
the user how to attach. The manager learns about the change through
its file watch; no prompt-the-manager step.

After parking, attach with:

```bash
tmux attach -t <target-tmux-session-name>
```

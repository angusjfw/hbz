---
name: claude-manager-shutdown
description: Shut down a Claude session from inside the worker pane — captures multi-window pane snapshots and a structured resume_state file for cold resume, records resumed_session_id, kills the tmux session, and keeps the registry entry for later resumption via the manager. Worker self-serves under the registry lock; manager observes via its watch. Use when pausing work for hours or days and you want the conversation reachable again, but don't want the tmux session hanging around.
---

# claude-manager-shutdown

`/claude-manager-shutdown`

Shutdown = kill the worker's tmux session, but keep the registry
entry so the session can be cold-resumed later. Sits between active
and wrap (final, removes the entry, writes journal).

This is a thin wrapper. The shared lifecycle flow lives at
`../claude-manager-end/FLOW.md`. Read it and run with `mode=shutdown`.

The worker handles shutdown itself: walks every window and pane in
the tmux session, writes a multi-window snapshot file, builds a
structured resume_state file capturing per-pane cwd, command and
Claude session id (for every Claude pane, not just the calling one),
rewrites its registry entry under the lock (adds `shutdown`,
`resumed_session_id`, `snapshot`, `resume_state`; drops
`tmux_session`), releases the lock, then kills the tmux session. The
manager learns about the change through its file watch.

To resume later, the manager's cold-resume flow rebuilds the whole
tmux session from the resume_state file. As an escape hatch, the
primary worker can also be brought back manually:

```bash
claude --resume <resumed_session_id>
```

The snapshot at
`~/.local/state/claude-manager/snapshots/<session-id>.txt` provides
context on where the session left off.

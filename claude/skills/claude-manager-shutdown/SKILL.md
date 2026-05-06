---
name: claude-manager-shutdown
description: Shut down a Claude session from inside the worker pane — captures pane snapshots, records the Claude conversation's resume id, kills the tmux container, and keeps the registry entry for later resumption via `claude --resume <id>`. Worker self-serves under the registry lock; manager observes via its watch. Use when pausing work for hours or days and you want the conversation reachable again, but don't want the tmux container hanging around.
---

# claude-manager-shutdown

`/claude-manager-shutdown`

Shutdown = kill the worker's tmux container, but keep the registry
entry so the Claude conversation can be resumed later. Sits between
park (keeps tmux alive) and wrap (final, removes the entry, writes
journal).

This is a thin wrapper. The shared lifecycle flow lives at
`../claude-manager-end/FLOW.md`. Read it and run with `mode=shutdown`.

The worker handles shutdown itself: captures every pane in its tmux
container to a snapshot file, resolves its own Claude conversation's
JSONL session id, rewrites its registry entry (adds `shutdown`,
`resumed_session_id`, `snapshot`; drops tmux fields), releases the
lock, then kills its tmux container. The manager learns about the
change through its file watch.

To resume later, from the worktree (or `cwd`):

```bash
claude --resume <resumed_session_id>
```

The snapshot at
`~/.local/state/claude-manager/snapshots/<session-id>.txt` provides
context on where the session left off.

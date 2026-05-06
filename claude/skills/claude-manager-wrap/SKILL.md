---
name: claude-manager-wrap
description: Wrap up a Claude session from inside the worker pane — captures snapshots, gathers any context worth carrying into the journal, marks the registry entry `wrap_requested`, and kills the tmux container. The manager observes via its watch and writes the journal entry per the project's schema, then removes the registry entry. Final state. Use when the work is genuinely done and you want it recorded.
---

# claude-manager-wrap

`/claude-manager-wrap`

Wrap = final close-out. Worker captures the session state and marks
the registry entry as ready to wrap; the manager then writes the
journal entry and removes the registry entry. This is the only
lifecycle transition that produces a journal record, so use it only
when the work is genuinely done.

This is a thin wrapper. The shared lifecycle flow lives at
`../claude-manager-end/FLOW.md`. Read it and run with `mode=wrap`.

The worker handles its phase itself:

1. Captures every pane to a snapshot file.
2. Resolves its Claude conversation's resume id (so the journal can
   record it).
3. Assesses what context is worth carrying into the journal — the
   conversation, recent git activity in the worktree, the snapshot. If
   something is missing or genuinely ambiguous, asks the user one
   focused question; otherwise doesn't interrupt.
4. Rewrites its registry entry under the lock: `wrap_requested: true`,
   `snapshot`, `resumed_session_id`, `notes` with the journal context.
5. Releases the lock and kills the tmux container.

The manager picks up `wrap_requested: true` via its watch, reads the
project's journal schema, writes the entry, and removes the registry
entry. If the manager isn't running, the marker persists; the next
manager invocation processes it.

Vocabulary on both sides: "wrap up", "complete", "close out", "finish"
all mean the same thing.

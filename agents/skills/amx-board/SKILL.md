---
name: amx-board
description: Review every agent session amx knows about and tidy up. Use when the user asks what sessions are open or running, what's stale, what's been left unrecorded, to review or tidy their sessions, or to catch the journal up across sessions. Works from any session, with or without a running manager.
---

# amx board

A one-off look across all agent sessions: what's live, what's parked,
what was left behind, and what never got recorded. You propose; the user
decides; `amx` does the changes. Don't investigate the work itself
(code, bugs, PRs); this is about the records.

## 1. Gather

```bash
amx ls -a                 # every entry: state (auto = registered by the hooks), busy, ticket, cwd
amx reconcile             # drift between the registry and tmux
amx log --untracked       # ended sessions that never belonged to a tracked entry
```

`amx reference` explains the fields and states if you need them.

## 2. Show the user one short list, grouped

- **Live**: active and paused entries, busy or waiting.
- **Worth keeping?** Auto entries that look like real work (a branch, a
  ticket-like name, long-running) rather than a quick question.
- **Dead**: entries whose tmux session is gone without a shutdown.
- **Overdue**: shut-down entries past their `resume_target`.
- **Unrecorded**: `amx log --untracked` sessions that look like real work
  (a branch other than the default, a worktree cwd) and have no journal
  entry. Skip quick ones.
- Anything else `amx reconcile` flagged.

For each item, say what you'd do. Nothing changes until the user picks.

## 3. Do what the user picks

- **Keep an auto entry:** `amx track <id> [--as <name>]`, then
  `amx reg set <name> ticket=… branch=…` if they give context.
- **Wrap** a live, dead or overdue entry: if the project has a journal,
  write the entry per its schema from `amx reg show <id>`, its snapshot
  and notes, and recent git activity in its cwd, carrying every session
  id and cwd as resume pointers. Then `amx wrap <id>`.
- **Record an unrecorded session:** journal entry from the log line
  (session id, cwd, branch) and the transcript
  (`amx transcripts <harness> <cwd> --grep <session id>` finds the file;
  read only what you need to summarise it).
- **Resume** a dead or shut-down entry: `amx rebuild <id>`. Tell the
  user the tmux session name to switch to.
- **Drop** an entry that isn't worth a record: `amx reg rm <id>` (the
  session log keeps a copy).
- Reconcile items: run the command it printed, if the user agrees.

Never kill a tmux session the user is working in; `amx` refuses shared
sessions, and so should you.

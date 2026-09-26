---
name: claude-manager-worker
description: Pause, shut down or wrap up this agent session from inside a claude-manager worker pane (Claude Code or pi). Use when the user says "pause", "park it", "unpause" (pause mode); "shut down", "shutdown", "kill this one", "drop tmux" (shutdown mode, keeps the session for a later resume); or "wrap up", "complete", "close out", "finish" (wrap mode, final, the manager then writes the journal entry). Takes the mode as its argument.
---

# claude-manager worker

You are an agent in a tmux session that claude-manager tracks. This skill
changes your own session's state. `cm` does all the mechanics; run the
steps in order and stop on any error.

## 1. Find your entry

```bash
cm whoami
```

Read `entry=` and `session=`. If `entry=` is empty, stop and tell the
user this tmux session isn't in the registry (show them the `cm whoami`
output). Don't guess an entry.

## 2. Pick the mode

- pause, park, unpause → **pause**
- shut down, kill this one, drop tmux, pause for days → **shutdown**
- wrap up, complete, close out, finish → **wrap**

If you can't tell which one the user means, ask.

## Pause

```bash
cm pause <entry>                 # toggles; or: cm pause <entry> on|off
cm pause <entry> on --reason "waiting on PR review"
```

Nothing is killed. Report what `cm` printed.

## Shutdown

Keeps the entry so the manager can bring the session back later.

1. Finish anything in flight and tell the user you're shutting down.
2. Run:

   ```bash
   cm shutdown <entry> [--resume-target "<date the user mentioned>"]
   ```

   This kills your own tmux session, so it must be your last action.
3. If it stops with "no session id for …": run
   `cm transcripts <harness> <cwd>` for that pane, pick the transcript
   whose content matches, and add `harness:` and `session_id:` lines under
   that pane in the resume_state file named in the error. Then rerun
   with `--force`. If you can't tell which transcript it is, ask the
   user.

## Wrap

Final. The manager writes the journal entry and removes the registry
entry afterwards.

1. Decide what the journal needs that the manager can't see: what was
   done, where it landed (branch, PR, commits), what's left, decisions
   worth keeping. Use the conversation and `git log` in your cwd. Ask the
   user one question only if something important is missing.
2. Run, with that context as one line:

   ```bash
   cm wrap <entry> --notes "<context>"
   ```

   This kills your own tmux session, so it must be your last action.

## If something fails

- "already shut down / wrap requested": the transition already happened.
  Tell the user and stop. Don't retry.
- "registry lock … held": another writer is stuck. Show the user the
  message; don't remove the lock yourself.
- Anything else: show the error verbatim and stop.

Formats and states: `~/.claude/skills/claude-manager/REFERENCE.md`.

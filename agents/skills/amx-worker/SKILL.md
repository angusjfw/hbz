---
name: amx-worker
description: Pause, shut down or wrap up this agent session from inside its tmux pane (Claude Code or pi), when amx tracks the session. Use when the user says "pause", "park it", "unpause" (pause mode); "shut down", "shutdown", "kill this one", "drop tmux" (shutdown mode, keeps the session for a later resume); or "wrap up", "complete", "close out", "finish" (wrap mode, final, with a journal entry). Takes the mode as its argument.
---

# amx worker

You are an agent in a tmux session that amx tracks. This skill changes
your own session's state. `amx` does the mechanics; run the steps in
order and stop on any error.

## 1. Find your entry

```bash
amx whoami
```

Read `entry=` and `session=`. If `entry=` is empty, stop and tell the
user this tmux session isn't in the registry (show them the output).
Don't guess an entry.

## 2. Pick the mode

- pause, park, unpause → **pause**
- shut down, kill this one, drop tmux, pause for days → **shutdown**
- wrap up, complete, close out, finish → **wrap**

If you can't tell which one the user means, ask.

## Pause

```bash
amx pause <entry>                 # toggles; or: amx pause <entry> on|off
amx pause <entry> on --reason "waiting on PR review"
```

Nothing is killed. Report what `amx` printed.

## Shutdown

Keeps the entry so the session can be brought back later.

1. Finish anything in flight and tell the user you're shutting down.
2. Run:

   ```bash
   amx shutdown <entry> [--resume-target "<date the user mentioned>"]
   ```

   This kills your own tmux session, so it must be your last action.
3. If it stops with "no session id for …": run
   `amx transcripts <harness> <cwd>` for that pane, pick the transcript
   whose content matches, and add `harness:` and `session_id:` lines under
   that pane in the resume_state file named in the error. Then rerun
   with `--force`. If you can't tell which transcript it is, ask the
   user.

## Wrap

Final: the entry is removed and the tmux session killed.

1. `amx reg show <entry>` and note the resume pointers: every session id
   (`resumed_session_id`, `worker:` lines) with its cwd.
2. If the project's rulebook describes a journal, write the entry now,
   per its schema: what was done, where it landed (branch, PR,
   commits), what's left, decisions worth keeping, the resume pointers,
   and the snapshot path `~/.local/state/amx/snapshots/<entry>.txt`. Use
   the conversation and `git log` in your cwd. Ask the user one question
   only if something important is missing. No journal: skip this step.
3. Run:

   ```bash
   amx wrap <entry> --notes "<what was done, and where the journal entry landed if you wrote one>"
   ```

   This kills your own tmux session, so it must be your last action.

## If something fails

- "already shut down" or "no registry entry": the transition already
  happened. Tell the user and stop. Don't retry.
- "registry lock … held": another writer is stuck. Show the user the
  message; don't remove the lock yourself.
- Anything else: show the error verbatim and stop.

`amx reference` prints the registry and file formats.

# Claude manager: surviving a tmux server death

Design for making a session recoverable at any moment, not only after a
clean shutdown. Companion to `2026-04-29-claude-manager-workflow.md`,
`2026-05-22-claude-manager-sessions-pivot.md` and
`2026-06-07-claude-manager-paused-state.md`.

## Goal

A tmux server dying should cost the running processes and nothing else.
The machine takes the processes either way; what has to survive is the
record of what was there — every window and pane, what ran in it, and
which Claude conversation each pane held — so the manager can rebuild it.

## The gap

Recovery state is written at exactly two moments, spawn and shutdown, and
nothing in between:

- The registry records spawn-time facts. Extra windows and panes created
  later are explicitly out of scope while the session lives — "user free
  space" in the spawn flow — so a coordinator's sub-workers are invisible
  to the registry for their whole lifetime.
- `resume_state` is written only by the shutdown flow.
- Cold resume *deletes* `resume_state` and the snapshot on the grounds
  that the live session supersedes them. A resumed session is therefore
  less recoverable than a shutdown one, and stays that way until it next
  shuts down cleanly.
- A freshly spawned session has no `resumed_session_id` at all. That
  field first appears at shutdown or wrap.

So the only durable record of a live session is its registry entry, and
that entry cannot say which conversation to resume.

## What the gap cost

An accidental machine shutdown killed the tmux server with no capture:

- **17 live registry entries** lost their tmux containers, none with a
  snapshot or `resume_state`.
- **2 of the 17** carried a `resumed_session_id`, both only because they
  had been shut down or reopened previously. The other **15** needed a
  content hunt through Claude's transcript files to find the right
  conversation.
- What made those 15 recoverable was the prose in `notes` — the recorded
  brief happened to be a good enough fingerprint to match a transcript by
  cwd and first prompt. Recovery rode on a field meant for humans.
- **Every non-primary Claude pane was unrecoverable as such.** Three were
  reconstructed as their own sessions because nothing recorded which
  session they had belonged to.

## Pane detection was broken independently

The documented way to identify a pane's foreground process —
`pgrep -P "$pane_pid" | head -1` — returned the wrong process on **all
27 live panes**. Claude Code spawns a shell of its own for tool calls, on
a separate pty, as a sibling of `claude` under the pane shell:

```
pane_pid 13200  (-zsh, ttys004)
  ├─ 13412  -zsh    ttys005   <- lower pid, so pgrep|head -1 picks this
  └─ 19378  claude --resume aa10322a-…
```

Followed literally, shutdown records `command: -zsh` with no
`claude_session_id` for every Claude pane, and cold resume sends `-zsh`
into the pane. Past resumes worked because the snippet was not followed
literally.

The fix is to keep the child sharing the pane's own tty. Verified correct
on all 27 panes, and it returns empty for a genuinely idle shell.

## Approach: record continuously, restore deliberately

Three layers, each owning what it is actually able to know.

1. **Identity at spawn.** Generate the conversation id instead of
   discovering it later.
2. **Structure continuously.** tmux-resurrect plus tmux-continuum keep a
   machine-readable record of the whole server, refreshed in the
   background.
3. **The worker set in the registry.** Only the manager knows which panes
   are workers under which session, so only the registry can record it.

Restore stays manager-owned throughout. The registry remains the
authority on lifecycle; the save file is evidence, not a competing
source of truth.

### 1. Identity at spawn

Spawn with `claude --session-id <uuid>` and write `resumed_session_id`
into the registry in the same action as `tmux_session`. This turns the
15 transcript hunts into 15 registry reads, and it puts the id in the
process's own argv, where any tool sampling `ps` picks it up for free.
`--session-id` applies to a new conversation only; cold resume keeps
passing `--resume <id>`, which is the same id.

### 2. Structure continuously — resurrect and continuum, save only

resurrect writes one tab-separated record per session, window and pane:
window layout, window name, pane cwd, pane title, active flags, and the
pane's full command line. continuum re-runs it on an interval.

Verified against the real server rather than assumed:

- The pane records carry `claude --effort … --resume <uuid>` verbatim, so
  the save file alone identifies each pane's conversation once spawns
  set an explicit id.
- The pane title is Claude's own summary of what the pane is doing, which
  makes an unregistered sub-pane legible during recovery.
- Pane contents cost ~53 KB gzipped for 22 panes, so capturing them is
  effectively free and preserves what was on screen.

Three caveats found by testing, all of which argue for treating the save
file as a best-effort supplement rather than a guarantee:

- **The bundled `ps` save-command strategy mis-saves Claude panes.** It
  prints every child of the pane shell, one per line, so Claude's tool
  shell lands in the save file as a stray record — 22 junk lines in a
  22-pane save. Its `grep "^$PANE_PID"` is also a prefix match, so pane
  pid 3378 matches an unrelated 33780. Replaced with a `pane-tty`
  strategy that matches ppid exactly and keeps the child on the pane's
  tty. resurrect falls back to the bundled one if the file is missing,
  so a plugin reinstall degrades rather than breaks.
- **Autosave needs an attached client.** continuum hooks a `#()` into
  `status-right`, which only runs when the status line redraws. A long
  detached stretch does not autosave. (`status-right-length 0`, which
  this config sets, does *not* prevent it — tested.)
- **Autosave silently does not install if another tmux server is
  running** when the config loads. continuum skips the hook to stop two
  servers overwriting each other's save file, and says nothing. A stray
  second server therefore disables the safety net invisibly.

Restore is disabled on both routes: `@continuum-restore off`, and
resurrect's own `prefix+C-r` unbound after tpm loads. Setting
`@resurrect-restore ''` does not work — resurrect reads an empty option
as unset and falls back to `C-r`. An unattended restore would recreate
sessions that were shut down deliberately and re-run `claude` in every
pane behind the manager's back.

### 3. The worker set in the registry

One new repeatable session field, following the existing `notes`
precedent of appearing more than once:

```
worker: <claude-session-id> cwd=<path> [label=<short description>]
```

One line per Claude pane beyond the primary, written when the pane is
created and dropped when it goes. The primary keeps using
`resumed_session_id`, which is load-bearing across the skill family and
documented as the read-the-registry-and-resume escape hatch.

Position is deliberately absent. `renumber-windows on` means window
indexes shift whenever a window closes, so an index recorded now is not
a reliable address later. The conversation id is stable, the cwd is
stable, and geometry comes from the save file — which is the division of
labour this design is built on.

Child registry entries were rejected: two entries would share one
`tmux_session`, and the worker-side flow finds its own entry by matching
`tmux_session` against its session name. Duplicates make that ambiguous,
which the flow handles by refusing to act.

### 4. Cold resume keeps its record

Cold resume stops deleting `resume_state` and the snapshot. It marks them
superseded and leaves them on disk, so a resumed session is never less
recoverable than a shutdown one.

### 5. Crash recovery

A new manager flow for "the tmux server is gone and nothing was
captured", which reads evidence in a fixed order **before** creating
anything:

1. The resurrect save file, for structure and per-pane commands.
2. The registry, for lifecycle and ownership.
3. The hook-fed agent-status store, for any Claude the save file missed.

The ordering matters for the third one. `agent-status` maps every Claude
session id to its pane, per tmux session, but it is a live status store
and not a recovery record: it drops any Claude whose recorded `pane_id`
no longer runs one. Pane ids do not survive a server restart, so the
first hook event in a recreated session wipes every *other* Claude id
from that entry. The pre-crash inventory is readable only until the first
session is recreated, so recovery copies the store aside first.

## Files touched

- `tmux/.tmux.conf` — tpm plugin block; resurrect/continuum options;
  `unbind -T prefix C-r` after the tpm `run`.
- `tmux/resurrect/save_command_strategies/pane-tty.sh` — new save-command
  strategy.
- `Makefile` — `tmux` target fetches tpm, resurrect and continuum, and
  symlinks the strategy into the plugin dir.
- `claude/skills/claude-manager/SKILL.md` — tty-scoped pane detection;
  `--session-id` at spawn and `resumed_session_id` recorded there;
  `worker:` field in the recognised and worker-writable field lists; cold
  resume keeps its record; new crash-recovery section; window indexes
  corrected for `base-index 1`.
- `claude/skills/coordinator-worker/SKILL.md` — registering each
  sub-worker becomes part of standing one up.

## Out of scope

- **tmux-assistant-resurrect.** It solves the same problem off the shelf
  — a `SessionStart` hook records each live conversation id, and a
  post-restore hook re-runs `claude --resume` per pane — and its
  detection reaches the same conclusion about avoiding `pgrep -P`. It is
  not adopted because its restore path targets panes by index and knows
  nothing about the registry, so it would race the manager's cold resume
  for ownership of bringing sessions back. Worth revisiting if the
  manager ever stops owning restore.
- **Auto-restore.** Deliberately off, per above.
- **Hardening the agent-status pruner** so it survives a server restart.
  That is session-leds work, and the copy-aside step covers the manager's
  need without it.
- **A second save destination.** One save file is a single point of
  failure, but continuum already rotates timestamped files and keeps
  `last` as a symlink, so history exists.

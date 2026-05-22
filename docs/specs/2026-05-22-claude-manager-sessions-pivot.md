# Claude manager: sessions pivot

Supersedes the Registry, Spawn, Park, Reconcile, Shutdown, Wrap and
Import sections of `2026-04-29-claude-manager-workflow.md`. The overall
framing (goals, non-goals, hard boundary on manager scope, registry as
shared channel, watch, knowledge work, idle detection, resource
awareness) is unchanged and continues to apply.

## What changes

A registry session was a tmux window inside the manager's tmux session,
with a separate "parked" state representing a window moved out to a
standalone tmux session. This pivots to: a registry session is one tmux
session, full stop. Each managed session is a sibling of the manager's
own tmux session on the same tmux server. The park concept retires.

## Why

Mapping registry sessions 1:1 to tmux sessions makes them first-class
tmux objects with their own window space. A task-run-style session can
carry a Claude worker in window 0 and a dev server, mock, watcher and
logs spread across further windows or panes, all under one addressable
tmux session. The earlier hesitation about clunky `switch-client` UX is
resolved by tmux's `prefix+w` picker, which gives an interactive list
across sessions and windows. Decoupling worker lifetime from the manager
and multi-client attach are nice side effects, not the driving goals.

## Core model

A registry session is 1:1 with a tmux session. By convention the tmux
session name equals the registry session id. This is enforced for
manager-spawned sessions: the spawn step creates the tmux session with
the registry id as its name, so they always match. The import flow
either renames an existing tmux session to match or adopts its name as
the session id.

The manager lives in whatever tmux session the user happened to launch
it in (default `tmux` gives auto-numbered names like `0`). That session
is distinguished only by housing the manager Claude. Ad-hoc windows for
quick work go there too, untracked by the registry.

Active vs shutdown is encoded by whether `tmux_session` is present on
the entry. No park state.

## Registry shape

Active entry, flat key:value:

```markdown
## eng-1234
ticket: ENG-1234
tmux_session: eng-1234
worktree: ~/code/.../eng-1234
branch: fix/eng-1234
started: 2026-05-22 09:00
last_touched: 2026-05-22 14:20
notes: ...
```

`tmux_session: <id>` is present iff the tmux session is alive.
`tmux_window_id`, the parked-only `tmux_session: <name>` signal, and
`claude_panes` all retire. The manager finds Claude panes on demand
(idle query, wrap fulfilment) via `tmux list-panes -s -t <session>`
plus content sniffing, rather than mirroring them in the registry.

Shutdown entry, same fields minus `tmux_session`, plus:

```markdown
shutdown: 2026-05-22
snapshot: ~/.local/state/claude-manager/snapshots/eng-1234.txt
resume_state: ~/.local/state/claude-manager/resume/eng-1234.md
resumed_session_id: abc-123
```

`resume_state` holds the authoritative per-window data for cold resume;
format below. `resumed_session_id` stays as a human convenience for the
primary worker (window 0 pane 0); reading the registry alone is enough
to fire a manual `claude --resume` for the common single-worker case.

Task-list prefixes collapse to `[active]`, `[shutdown]`,
`[wrap requested]`. `[parked]` retires.

## Spawn flow

1. Clarify ticket, worktree, branch (unchanged).
2. Create the worktree if wanted (unchanged).
3. Pre-check name collision:

   ```bash
   tmux has-session -t "$session_id" 2>/dev/null
   ```

   If a session by that name exists, surface and ask: import or pick a
   new id. Do not silently take over.
4. Create the tmux session:

   ```bash
   tmux new-session -d -s "$session_id" -n "$session_id" -c "$cwd"
   ```

   `-d` keeps the focus rule (no stealing the user's view). `-n` names
   window 0 after the session id for tidiness; the user is free to
   rename later.
5. Start Claude in window 0 pane 0 using the existing poll-the-TUI
   kickoff pattern. The poll-marker check must match what the current
   TUI renders; capture first and adapt the regex.
6. Write the registry entry with `tmux_session: $session_id`. Add task
   as `[active]`.
7. Hand the user the switch handle.

No `tmux_window_id` capture step. No window-id-to-location lookups
anywhere downstream.

## Switch UX

Primary: `prefix+w` picker, an interactive list across sessions and
windows. Direct: `tmux switch-client -t <session-id>`. Manager hands
back the session id; the user navigates.

## Import an existing tmux session

1. `tmux has-session -t <existing-name>` to confirm.
2. If the existing tmux session name equals the desired registry
   session id, register as-is. If different, either rename it
   (`tmux rename-session`) or adopt the existing name as the session
   id.
3. Ask for missing context (ticket, branch, worktree, anything else
   worth recording).
4. Write the entry with `tmux_session: <name>`. Add task as `[active]`.

The old "[parked] on import by convention" rule retires alongside park
itself.

## Shutdown

Shutdown = kill the tmux session, keep the registry entry for cold
resume. Sits between active and wrap. Flexible wording: "shutdown",
"kill that one", "pause it", "drop tmux".

### Mechanics

1. Discover structure:

   ```bash
   tmux list-windows -t "$session" \
     -F '#{window_index} #{window_name} #{window_layout}'
   tmux list-panes -t "$session":<w> \
     -F '#{pane_index} #{pane_current_path} #{pane_current_command}'
   ```

   Run the panes query per window, capturing every pane.
2. Find Claude session ids for every pane that looks like Claude (by
   `pane_current_command` or content sniff). For each candidate, grep
   its recent capture for a distinctive phrase and match against
   `~/.claude/projects/<encoded-cwd>/*.jsonl`. Where shared project
   dirs make multiple sessions ambiguous, fall back to mtime plus
   claim-tracking. Encoding rule unchanged from today: every `/`, `.`,
   and `_` in the absolute cwd becomes `-`, with the leading `/`
   producing a leading `-`.
3. Capture pane snapshots. Every pane in every window into one
   snapshot file, separated by `--- window <w> pane <p> ---` markers.
4. Build the resume_state file at
   `~/.local/state/claude-manager/resume/<session-id>.md` per the
   format below.
5. Update registry under lock: remove `tmux_session`; add `shutdown`,
   `snapshot`, `resume_state`, `resumed_session_id` (primary worker =
   window 0 pane 0). Append a note: "Tmux killed <date>; resume via
   the manager."
6. Kill the tmux session:

   ```bash
   tmux kill-session -t "$tmux_session"
   ```

### resume_state file format

Markdown, same idiom as the registry. One window per
`## window <n>: <name>` block; one pane per `### pane <n>` sub-block.
Layout captured as the opaque `#{window_layout}` string from tmux,
which round-trips via `tmux select-layout`. `cwd:` is per-pane because
splits can land in different dirs after `cd`. For panes whose
`pane_current_command` is a shell (`bash`, `zsh`, `fish`), `command:`
is left empty; auto-replaying an idle shell is noise.

```markdown
# Resume state: eng-1234

shutdown: 2026-05-22

## window 0: claude
layout: 5fe4,200x50,0,0,0

### pane 0
cwd: ~/code/.../eng-1234
command: claude --resume abc-123
claude_session_id: abc-123

## window 3: dev
layout: 9a3c,200x50,0,0{100x50,0,0,1,99x50,101,0,2}

### pane 0
cwd: ~/code/.../mock
command: yarn mock

### pane 1
cwd: ~/code/.../eng-1234
command: yarn dev

### pane 2
cwd: ~/code/.../eng-1234
command: yarn test --watch
```

### Cold resume

New manager operation. No equivalent in the previous design.

1. Read the registry shutdown entry plus resume_state file.
2. Pre-check name collision: `tmux has-session -t "$session_id"` must
   return non-zero. Fail loud if alive.
3. Recreate the session window-by-window:

   ```bash
   # First window
   tmux new-session -d -s "$session_id" \
     -n "$window0_name" -c "$pane0_cwd"
   # Per subsequent window:
   tmux new-window -d -t "$session_id": -n "$name" -c "$pane0_cwd"
   ```

4. Add splits for additional panes, one per recorded pane n>0:

   ```bash
   tmux split-window -d -t "$session_id":<w> -c "$pane_n_cwd"
   ```

5. Apply the captured layout to restore geometry:

   ```bash
   tmux select-layout -t "$session_id":<w> "$layout"
   ```

6. Send the recorded command per pane, skipping empty ones:

   ```bash
   tmux send-keys -t "$session_id":<w>.<p> "$command" Enter
   ```

7. Update registry under lock: add `tmux_session: $session_id`; remove
   `shutdown`, `snapshot`, `resume_state`, `resumed_session_id`;
   update `last_touched`. Delete the on-disk snapshot plus
   resume_state files (live session supersedes them). Task-list
   prefix `[shutdown]` to `[active]`.

The manual escape hatch stays available: a user can always run
`claude --resume <id>` from the worktree to bring the primary worker
back without the surrounding scaffold. That route doesn't update the
registry; subsequent reconcile catches the drift.

### Trigger paths

- Manager-driven, user ask: full mechanics above.
- Worker self-shutdown (`/claude-manager-shutdown`): worker does steps
  1-6 itself. It has direct access to its own JSONL, so identifying
  the calling Claude is unambiguous. For multi-Claude sessions (forked
  workers in other windows), the self-shutdown skill needs to broaden
  to capture every Claude pane, not just the calling one. The worker
  uses the same `tmux list-panes -s -t <session>` plus content-sniff
  path the manager would.

## Wrap

Wrap = final close-out. Journal entry written per project schema;
registry entry removed; tmux session killed. Flexible wording: "wrap
up", "complete", "close out", "finish".

### Mechanics

1. Capture `tmux_session` into a shell var. The registry rewrite below
   removes it.
2. Capture snapshots of every pane in every window into the snapshot
   file (same multi-window markers as shutdown).
3. Write the journal entry per the project's schema, using snapshot
   plus notes. If notes are thin and there's no obvious narrative from
   snapshot plus recent git activity in the worktree, ask a focused
   question before writing.
4. Mark task `completed`, remove from list.
5. Remove the entry from the registry.
6. Kill the tmux session:

   ```bash
   tmux kill-session -t "$tmux_session"
   ```

### Trigger paths

- Manager-driven, user ask: full mechanics above.
- Worker self-wrap (`/claude-manager-wrap`): worker pre-captures
  snapshot, resolves its `resumed_session_id`, gathers notes, sets
  `wrap_requested: true`, kills the tmux session. Manager picks up the
  marker via watch and runs steps 3-5. Step 6 already done.
- Manager-not-running case: marker persists; next manager invocation
  processes it during reconcile.

The asymmetric worker/manager wrap split (worker does pre-capture plus
kill; manager does journal plus remove) is preserved here. The "shift
fully to the `wrap_requested` marker flow" question from the followups
file is a separate refactor, not part of this pivot.

## Reconcile

- Active entry: `tmux has-session -t <tmux_session>`. Alive or not.
  Not alive: surface and ask (finished, shutdown unexpectedly, or
  unknown).
- Entry without `tmux_session`: check `shutdown` (leave alone),
  `wrap_requested: true` (run wrap fulfilment if watch missed it),
  else ask.
- tmux session present but no matching registry entry: ask import or
  ignore. Don't silently take ownership.
- If many entries fail lookup at once, suspect a tmux server restart
  and surface aggregately before changing any state.

The stale-handle-points-at-the-wrong-thing risk goes away with this
pivot. `tmux_session` is a user-namespace name, not a server-lifetime
numeric handle. A server restart can leave sessions absent but cannot
recycle a session id to point at something unrelated.

## Watch

Unchanged. mtime poll on the registry file. PID file at
`~/.local/state/claude-manager/watch.<sanitised-mgr-pane>.pid`,
`Monitor`-consumed stdout. Self-writes-don't-double-fire is unaffected.

## Task list hygiene

Prefix set: `[active]`, `[shutdown]`, `[wrap requested]`. Sync triggers:

- Spawn or import: add task `in_progress` with `[active]`.
- Wrap-requested seen: `[wrap requested]`.
- Wrap fulfilled: `completed`, then remove.
- Shutdown: `[shutdown]`.
- Cold resume: `[shutdown]` to `[active]`.
- Ticket, notes, branch changes: update description.

## Worker write-permissions

Own entry only. Allowed fields are the same as today minus
`tmux_session` add/remove (no park = no need to touch it after spawn).
Wrap and shutdown fields still permitted. Cold resume is manager-only;
workers can't bring back their own dead session.

## What deliberately doesn't change

- Watch lifecycle.
- Knowledge work (journal, wiki, harness notes per project schema).
- Idle-detection query.
- Resource awareness.
- Manager hard boundary (meta-work only; substantive work always
  delegated to a worker).
- Header-line manager registration on invocation.
- The mkdir-lock convention for full-file registry rewrites.

## What retires

- The Park lifecycle transition entirely.
- The `tmux_window_id` field on registry entries.
- The parked-only `tmux_session: <name>` signal (replaced by an
  always-present-when-active `tmux_session: <session-id>`).
- The `claude_panes` field.
- The window-id-by-`@N` lookup pattern
  (`tmux list-windows -a -F '#{window_id} ...' | awk ...`).
- The Renumber section (no managed window list to renumber; ad-hoc
  windows in the manager's own tmux session are user free space).
- The `/claude-manager-park` skill.

## Migration

The user has confirmed all current sessions are either shutdown or
wrapped; no active or parked entries to migrate. The pivot lands on a
quiescent registry. Implementation can drop all `tmux_window_id`
support without a compatibility shim. The 2026-04-29 spec's `Importing
an existing tmux session` section is replaced by the shorter version
above.

## Followups touched

- **#2 stale `@N` references after tmux server restart**: dissolves.
  Removed from the followups file on completion.
- **#7 switch manager workflow from tmux windows to tmux sessions**:
  this work. Retires on completion.
- **Shutdown notes redundancy**: partially resolved. The shutdown
  notes line is reworded to "resume via the manager"; the standalone
  `resumed_session_id` field continues to exist as a primary-worker
  convenience that overlaps minimally with the resume_state file.
  Tighten or remove from the followups file based on outcome.
- **#1 mid-wrap resume-replay**, **#3 edit-tool blindness**, **#4
  asymmetric wrap roles**: unchanged. Same issues exist in the new
  model.
- **#5 over-prompting on spawn briefs**, **#6 hbz convention
  awareness**: orthogonal, untouched.

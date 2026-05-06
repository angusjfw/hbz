# Worker session lifecycle: park, shutdown, wrap

Shared flow for the three worker-side slash commands:

- `/claude-manager-park` — `mode=park`
- `/claude-manager-shutdown` — `mode=shutdown`
- `/claude-manager-wrap` — `mode=wrap`

Each top-level skill is a thin wrapper that invokes this flow with a
specific mode. The mechanics live here so the three modes don't drift.

In claude-manager terminology:

- **Park** moves the worker's tmux container out of the manager's window
  list into a standalone tmux session. The session keeps running.
  Reversible.
- **Shutdown** kills the tmux container but keeps the registry entry, so
  the conversation can be resumed later via `claude --resume <id>`.
- **Wrap** is final. Snapshot, journal entry (written by the manager from
  the worker's `wrap_requested` marker), registry entry removed.

Workers run all three themselves under the registry lock. The manager
observes via its registry watch and handles any follow-up that lives on
the manager side (journal write for wrap, window renumbering after a
worker kills a tmux window). See `claude-manager/SKILL.md` for the
manager-side mechanics; the modes use the same vocabulary on both sides.

## Common preamble

All three modes start the same way.

1. Resolve worker location:

   ```bash
   pane="$TMUX_PANE"
   src_session=$(tmux display-message -p -t "$pane" '#S')
   src_window=$(tmux display-message -p -t "$pane" '#I')
   src_pane=$(tmux display-message -p -t "$pane" '#P')
   window_name=$(tmux display-message -p -t "$pane" '#W')
   ```

2. Acquire the registry lock (cross-platform; `flock` is Linux-only):

   ```bash
   _reg="$HOME/.local/state/claude-manager/sessions.md"
   _lock="${_reg}.lock"
   t=0
   while ! mkdir "$_lock" 2>/dev/null; do
     sleep 0.1; t=$((t+1))
     [ $t -ge 100 ] && { echo "lock held >10s, aborting"; exit 1; }
   done
   trap "rmdir '$_lock'" EXIT INT TERM
   ```

3. Find the worker's own session entry. Match its tmux address against
   recorded fields:

   - If `tmux_window` equals `<src_session>:<src_window>` — the worker
     lives as a window inside another tmux session (typically the
     manager's). Park is the natural mode.
   - If `tmux_session` equals `<src_session>` — the worker already lives
     in its own tmux session (previously parked, or imported). Park is
     a no-op; shutdown and wrap apply.

   If neither matches any entry, surface what was searched for and what
   the registry actually holds, then stop. Don't guess.

After the preamble, branch on `mode`.

## Mode: park

Move the worker's tmux window into a standalone tmux session.

If `tmux_window` isn't set on the entry (worker is already in a
standalone tmux session), surface the entry and stop — nothing to park.

```bash
target="${1:-$window_name}"
tmux new-session -d -s "$target" -n placeholder
tmux move-window -d -s "${src_session}:${src_window}" -t "${target}:0" -k
```

Rewrite the entry under the lock:

- Drop `tmux_window`.
- Add `tmux_session: $target`.
- Update `last_touched`.
- Preserve all other fields and unknown keys.

Release the lock. Tell the user `tmux attach -t $target`.

The manager will see the change via its watch and update its task list.
No prompt to the manager pane is needed.

## Mode: shutdown

Kill the tmux container; preserve the registry entry so the Claude
conversation can be resumed via `claude --resume <id>`.

1. **Capture every pane** in the worker's tmux container — not just
   `claude_panes`. Other panes may hold dev servers, shells, or context
   worth keeping.

   ```bash
   container="${src_session}:${src_window}"   # tmux_window case
   # or: container="${src_session}"           # tmux_session case
   snapshot="$HOME/.local/state/claude-manager/snapshots/<session-id>.txt"
   mkdir -p "$(dirname "$snapshot")"
   tmux list-panes -t "$container" -F '#{pane_index}' | while read p; do
     echo "--- pane $p ---"
     tmux capture-pane -p -J -t "${container}.${p}" -S -500
     echo
   done > "$snapshot"
   ```

2. **Resolve `resumed_session_id`.** If the registry entry already has
   this field, reuse it and skip the rest of this step —
   `claude --resume <id>` continues writing to the same JSONL, so the
   id is stable across resume cycles.

   Otherwise, the worker is itself a Claude session, so the
   most-recently-modified JSONL under the project's session dir *is*
   this session — it's being written to right now.

   ```bash
   cwd="$(pwd)"   # or the entry's worktree if set
   encoded=$(echo "$cwd" | sed 's|[/._]|-|g')
   proj_dir="$HOME/.claude/projects/$encoded"
   jsonl=$(ls -t "$proj_dir"/*.jsonl 2>/dev/null | head -1)
   resumed_session_id="$(basename "${jsonl%.jsonl}")"
   ```

   Encoding: every `/`, `.` and `_` in the absolute cwd is replaced
   with `-`. Don't strip the leading `/` — it produces a leading `-`
   on the directory name, which is correct (e.g.
   `/Users/foo.bar/code/my_service` →
   `-Users-foo-bar-code-my-service`).

   Fallback if the cwd encoding doesn't match any dir (e.g. cwd is a
   symlink): grep the snapshot for a distinctive phrase and match
   against the candidate JSONL files. If still ambiguous, surface the
   candidates and stop.

3. **Rewrite the entry** under the lock:

   - Add `resumed_session_id`, `snapshot: <path>`, `shutdown: <today>`.
     Add `resume_target` if the user mentioned a date.
   - Update `last_touched`.
   - Append a `notes` line: "Shutdown by self <date>; resume via
     `claude --resume <id>` from the worktree/cwd."
   - Drop `tmux_session`, `tmux_window`, `claude_panes`.
   - Preserve all other fields and prose.

4. **Release the lock.**

5. **Kill the tmux container.** This kills the worker's own pane, so
   the registry write must already have landed:

   - `tmux_window` case: `tmux kill-window -t "${src_session}:${src_window}"`.
     Sibling windows are renumbered by the manager on its next
     interaction (cosmetic).
   - `tmux_session` case: `tmux kill-session -t "${src_session}"`.

To resume later, from the worktree (or `cwd`):

```bash
claude --resume <resumed_session_id>
```

## Mode: wrap

Final state — work is done. Two-phase: worker captures and marks; manager
fulfils the journal write.

**Worker phase:**

1. **Capture pane snapshots** as in Shutdown. Same path convention.

2. **Resolve `resumed_session_id`** as in Shutdown — the journal entry
   typically wants it.

3. **Assess journal context.** The worker has the conversation, recent
   git activity in the worktree, and the snapshot. If the picture is
   already clear, proceed. If something is genuinely missing or
   ambiguous, ask the user one focused question — don't interrupt for
   anything routine. Whatever context is captured goes into the entry as
   `notes` (single-line) or as a path to a notes file the manager can
   read.

4. **Rewrite the entry** under the lock:

   - Add `wrap_requested: true`.
   - Add `snapshot: <path>`, `resumed_session_id: <id>`.
   - Update `last_touched`.
   - Add or update `notes` with the journal context.
   - Leave `tmux_session` / `tmux_window` / `claude_panes` in place —
     the manager cleans those up after the journal write.

5. **Release the lock.**

6. **Kill the tmux container** as in Shutdown.

**Manager phase** (driven by the watch — no action needed from the
worker after step 6): the manager's reaction loop sees `wrap_requested:
true`, reads the project's journal schema, reviews the snapshot and
notes, asks the user a focused question if the picture is thin, writes
the journal entry, removes the registry entry, renumbers sibling
windows if `tmux_window` was set, and marks the task list completed.

If the manager isn't running, the marker persists. The next manager
invocation picks it up.

## Failure modes

Every failure surfaces evidence and stops. No silent fallthrough.

- **Registry missing or unreadable.** Print the path and the read
  error. The manager hasn't started, or the file was moved.
- **No matching entry for the worker's location.** Print the worker's
  tmux address and the registry's session entries (id + tmux fields).
  Likely the manager hasn't recorded this session, or the worker is in
  a tmux session unrelated to claude-manager.
- **Lock contention beyond 10 s.** Print the lock path and the holder
  (`ls -ld` to show owner). Don't force-release; ask the user.
- **Park: worker already in standalone tmux session.** Print the entry
  and stop.
- **Shutdown/wrap: JSONL resolution fails.** Print the project dir
  searched and any candidate JSONL files. Ask the user to identify the
  right one or skip the `resumed_session_id` field.
- **tmux move/kill fails.** Surface the tmux error verbatim. Common
  causes: target tmux session name already in use (park), invalid
  target (shutdown).

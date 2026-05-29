# Worker session lifecycle: shutdown, wrap

Shared flow for the two worker-side slash commands:

- `/claude-manager-shutdown` — `mode=shutdown`
- `/claude-manager-wrap` — `mode=wrap`

Each top-level skill is a thin wrapper that invokes this flow with a
specific mode. The mechanics live here so the two modes don't drift.

In claude-manager terminology:

- **Shutdown** kills the tmux container but keeps the registry entry, so
  the conversation can be resumed later via `claude --resume <id>`.
- **Wrap** is final. Snapshot, journal entry (written by the manager from
  the worker's `wrap_requested` marker), registry entry removed.

Workers run both themselves under the registry lock. The manager
observes via its registry watch and handles any follow-up that lives on
the manager side (journal write for wrap). See
`claude-manager/SKILL.md` for the manager-side mechanics; the modes use
the same vocabulary on both sides.

## Lock pattern

Every registry rewrite happens under a `mkdir` lock. Acquire just
before the rewrite, release just after — keep the critical section
tight, not spanning snapshot capture or tmux moves.

```bash
_reg="$HOME/.local/state/claude-manager/sessions.md"
_lock="${_reg}.lock"
t=0
while ! mkdir "$_lock" 2>/dev/null; do
  sleep 0.1; t=$((t+1))
  [ $t -ge 100 ] && { echo "lock held >10s, aborting"; exit 1; }
done
# read registry, mutate own entry, write registry (Edit tool is fine)
rmdir "$_lock"
```

Each Bash tool call runs in a fresh shell — `trap`-based release
does NOT survive across calls, so don't rely on it. The acquire and
release happen in different Bash calls in practice (Edit is in
between); if anything errors between them, the lock is held until
explicitly released. Recovery: `rmdir ~/.local/state/claude-manager/sessions.md.lock`
and retry.

## Common preamble

All three modes start the same way.

1. Resolve worker location:

   ```bash
   pane="$TMUX_PANE"
   src_session=$(tmux display-message -p -t "$pane" '#S')
   ```

2. Read the registry (no lock — reads are full-file, last-write-wins
   is fine for this flow). Find the worker's own session entry by
   matching its `tmux_session` against `$src_session`.

   If no entry matches, surface what was searched for and what the
   registry actually holds, then stop. Don't guess. If more than one
   matches (shouldn't happen — `tmux_session` is unique by
   construction), surface both and stop.

After the preamble, branch on `mode`.

## Mode: shutdown

Kill the tmux session; preserve the registry entry so the session can
be cold-resumed later via the manager. The full mechanics are
mirrored in `claude-manager/SKILL.md` under Shutdown — this section
covers the worker-side specifics.

1. **Discover structure.** Walk every window and every pane in the
   tmux session:

   ```bash
   tmux list-windows -t "$src_session" \
     -F '#{window_index} #{window_name} #{window_layout}'
   # per window:
   tmux list-panes -t "$src_session":<w> \
     -F '#{pane_index} #{pane_current_path} #{pane_current_command}'
   ```

2. **Capture pane snapshots** for every pane in every window into one
   snapshot file, with `--- window <w> pane <p> ---` markers:

   ```bash
   snapshot="$HOME/.local/state/claude-manager/snapshots/<session-id>.txt"
   mkdir -p "$(dirname "$snapshot")"
   tmux list-windows -t "$src_session" -F '#{window_index}' | while read w; do
     tmux list-panes -t "$src_session":$w -F '#{pane_index}' | while read p; do
       echo "--- window $w pane $p ---"
       tmux capture-pane -p -J -t "${src_session}:${w}.${p}" -S -500
       echo
     done
   done > "$snapshot"
   ```

3. **Resolve Claude session ids for every Claude pane.** The calling
   worker is itself a Claude session; the most-recently-modified
   JSONL under its project dir is this session by definition. For
   other Claude panes (forked workers in other windows), do the same
   per-pane resolution as in the manager-side mechanics (step 3 of
   `claude-manager/SKILL.md` Shutdown).

   ```bash
   # for each pane that looks like Claude:
   cwd="<pane_current_path>"
   encoded=$(echo "$cwd" | sed 's|[/._]|-|g')
   proj_dir="$HOME/.claude/projects/$encoded"
   jsonl=$(ls -t "$proj_dir"/*.jsonl 2>/dev/null | head -1)
   claude_session_id="$(basename "${jsonl%.jsonl}")"
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

   Shared cwd: if multiple Claude panes share the same cwd, `ls -t`
   may pick another pane's JSONL. Verify by grepping the snapshot
   section for that window/pane for a phrase only present in this
   pane's JSONL; if no JSONL contains it, surface the candidates and
   ask.

4. **Build the resume_state file** at
   `~/.local/state/claude-manager/resume/<session-id>.md`. Markdown,
   one window per `## window <n>: <name>` block with a `layout:` field,
   one pane per `### pane <n>` sub-block with `cwd:`, `command:`, and
   on Claude panes `claude_session_id:`. For panes whose
   `pane_current_command` is a shell (`bash`, `zsh`, `fish`), leave
   `command:` empty — auto-replaying an idle shell on resume is noise.
   See `claude-manager/SKILL.md` Shutdown for the full example.

5. **Acquire the lock, rewrite the entry, release the lock** (see
   Lock pattern). The rewrite:

   - Adds `resumed_session_id` (primary worker = window 0 pane 0).
   - Adds `snapshot: <path>`, `resume_state: <path>`,
     `shutdown: <today>`.
   - Adds `resume_target` if the user mentioned a date.
   - Updates `last_touched`.
   - Appends a `notes` line: "Shutdown by self <date>; resume via the
     manager."
   - Drops `tmux_session`.
   - Preserves all other fields and prose.

6. **Kill the tmux session.** Only do this after the lock has been
   released and the registry write has landed; this kills the
   worker's own pane, so any remaining work must be done first.

   ```bash
   tmux kill-session -t "$src_session"
   ```

To resume later, the manager's cold-resume flow rebuilds the whole
tmux session from the resume_state file. A manual
`claude --resume <resumed_session_id>` from the worktree still works
as an escape hatch for the primary worker only.

## Mode: wrap

Final state — work is done. Two-phase: worker captures and marks; manager
fulfils the journal write.

**Worker phase:**

1. **Capture pane snapshots.** Walk every window and every pane in
   the tmux session, concatenating into one snapshot file with
   `--- window <w> pane <p> ---` markers (same format as shutdown):

   ```bash
   snapshot="$HOME/.local/state/claude-manager/snapshots/<session-id>.txt"
   mkdir -p "$(dirname "$snapshot")"
   tmux list-windows -t "$src_session" -F '#{window_index}' | while read w; do
     tmux list-panes -t "$src_session":$w -F '#{pane_index}' | while read p; do
       echo "--- window $w pane $p ---"
       tmux capture-pane -p -J -t "${src_session}:${w}.${p}" -S -500
       echo
     done
   done > "$snapshot"
   ```

2. **Resolve `resumed_session_id`** as in Shutdown — the worker is
   itself a Claude session, so the most-recently-modified JSONL
   under its project dir is this session by definition. The journal
   entry typically wants this id.

3. **Assess journal context.** The worker has the conversation, recent
   git activity in the worktree, and the snapshot. If the picture is
   already clear, proceed. If something is genuinely missing or
   ambiguous, ask the user one focused question — don't interrupt for
   anything routine. Whatever context is captured goes into the entry as
   `notes` (single-line) or as a path to a notes file the manager can
   read.

4. **Acquire the lock, rewrite the entry, release the lock** (see
   Lock pattern). The rewrite:

   - Adds `wrap_requested: true`.
   - Adds `snapshot: <path>`, `resumed_session_id: <id>`.
   - Updates `last_touched`.
   - Adds or updates `notes` with the journal context. If `notes` is
     already a path (workers and managers may set it to a journal
     file), append to that file instead of overwriting the field.
     Whenever notes reference the resume id, write the full
     `resumed_session_id` — never `<prefix>-...`.
   - Drops `tmux_session` — the session is about to be killed.

5. **Kill the tmux session** after the lock has been released and the
   registry write has landed. This kills the worker's own pane, so
   any remaining work must be done first:

   ```bash
   tmux kill-session -t "$src_session"
   ```

**Manager phase** (driven by the watch — no action needed from the
worker after step 5): the manager's reaction loop sees `wrap_requested:
true`, reads the project's journal schema, reviews the snapshot and
notes, asks the user a focused question if the picture is thin, writes
the journal entry, removes the registry entry, and marks the task
list completed.

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
- **Shutdown/wrap: JSONL resolution fails.** Print the project dir
  searched and any candidate JSONL files. Ask the user to identify the
  right one or skip the `resumed_session_id` field.
- **tmux kill fails.** Surface the tmux error verbatim. Common cause:
  invalid target (shutdown).

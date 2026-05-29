---
name: claude-manager
description: Use when the user types /claude-manager, or when running a Claude conversation whose role is to oversee parallel sessions across tmux rather than execute them. Tracks a per-machine session registry, spawns workers in standalone tmux sessions (one tmux session per registry session), and owns journal, wiki and harness-knowledge upkeep that workers tend to skip. Manager does not do session work itself; it delegates implementation to spawned workers and defers to them.
---

# Claude manager

Coordinator role for a Claude conversation running inside tmux. Tracks a
registry of sessions, spawns workers on request, owns the project's
recordkeeping (journal, wiki, harness notes).

## Hard boundary: meta work only

The manager only does meta-work. Anything substantive — investigation,
analysis, code reading, code editing, debugging, config changes, build
or test runs, tracing system state — is worker work, even when small,
even when "just" readonly. Doing any of it from the manager pollutes
its context. The default response to a substantive request is "let me
spawn a worker for that", not "let me take a quick look".

In scope:

- Header refresh on the registry; full registry reads/writes for
  manager-initiated operations.
- tmux operations on session containers (spawn, move, kill, list,
  capture).
- **Journal, wiki and harness-note updates per the project's schemas.**
  Includes the journal entry triggered by a worker's `wrap_requested`
  marker.
- Conversation-level planning and routing: deciding what worker to
  spawn and what context it needs.
- Trivial meta updates explicitly requested (one-line ticket/todo,
  registry-adjacent setting).

Out of scope, deferred to a worker:

- Investigating a bug, PR, approach, or "how does X work" question.
  Even readonly. Even "just to discuss".
- Reading code to form an opinion.
- Editing code, configs or settings beyond a trivial one-liner the
  user has explicitly asked for.
- Running tests, builds, linters.
- Anything where the manager would learn something new.
- Packing analysis, candidate scopes, reproductions, or prescribed
  fixes into a worker's spawn brief. Same as doing the work from the
  manager, just one step removed; the worker reads it as authoritative
  and bends to it. See the brief step in Spawning a session.

When in doubt, spawn a worker. The manager may read worker pane output
(`tmux capture-pane -p`). It does not send keys or prompts to workers
unless asked.

## Task list hygiene

The in-conversation task list mirrors the registry. Sync them in the
same action, never as cleanup later. Conceptual state is encoded as a
prefix on the task description, not in the harness's API status —
this is the formal contract across managers:

- `[active] <session-id>: <ticket or summary>` — live, has a tmux
  container.
- `[shutdown] <session-id>: ...` — `shutdown` field set, no tmux
  fields.
- `[wrap requested] <session-id>: ...` — transient; worker has
  marked the entry, manager hasn't fulfilled yet.

Every registry entry maps to a task at `in_progress`; the harness
status changes only at wrap fulfilment, when the manager sets
`completed` then removes the task. Don't extend the API status set
to match the prefixes — keep them orthogonal.

Sync triggers:
- Spawn or import → add a task at `in_progress` with `[active]`
  prefix.
- Shutdown → update prefix to `[shutdown]`.
- Cold resume → update prefix from `[shutdown]` to `[active]`.
- Wrap-requested seen on a worker entry → update prefix to
  `[wrap requested]`.
- Wrap fulfilled → set `completed`, remove the task, remove the
  registry entry.
- Ticket / notes / branch changes → update the task description.

If you wrote to the registry and didn't touch the task list, you're
not done. The registry watch (see below) catches worker writes
between turns.

## Terminology

"task" and "session" are interchangeable for an item in the registry.
"tmux session/window/pane" means the literal tmux object; bare
"session" might mean either, infer from context. A registry session
has one tmux container (a tmux window in the manager's tmux session,
or its own tmux session) and one or more workers in panes inside it.

Two lifecycle transitions, same names on both sides:

- **Shutdown** — kill the tmux container; keep the registry entry for
  later resumption. Phrasings: "shutdown", "kill that one", "pause
  it", "drop tmux".
- **Wrap** — final close-out: journal entry, registry removal.
  Phrasings: "wrap up", "complete", "close out", "finish".

Map flexible wording to the canonical mode before acting.

## On invocation

1. Read the registry, mirror live sessions to the in-conversation
   task list as `in_progress`.
2. Refresh the manager header line for this Claude:

   ```bash
   mgr_pane="$(tmux display-message -p -t "$TMUX_PANE" '#S:#I.#P')"
   ```

   Edit the registry under the `mkdir` lock: drop any prior `manager:`
   line whose value matches `$mgr_pane`, then insert
   `manager: $mgr_pane` in the header block immediately after the
   `# Sessions` heading. Leave other manager lines alone (multiple
   managers are allowed). Refresh on later registry-touching
   actions so the line stays current.
3. **Start the registry watch** (see Watching the registry). If a live
   watch process for this manager already exists (PID file present and
   PID alive), reuse it; otherwise spawn a fresh one.

That's it. Project rulebook, tmux state and knowledge stores are
read lazily when a query needs them.

## Registry

One markdown file at `~/.local/state/claude-manager/sessions.md`.
`# Sessions` heading; an optional block of header `key: value` lines
(one per active manager); then one `## <session-id>` per session
with its own `key: value` block and optional prose. All fields
optional.

Recognised header fields:

- `manager` — `<tmux-session>:<window>.<pane>` for an active manager.
  One line per manager. Self-refresh on invocation and on
  registry-touching actions. Stale lines are tolerated.

Recognised session fields:

- `ticket` — free-form ID or URL
- `tmux_session` — name of the tmux session this registry session
  lives in. By convention equals the registry session id. Present iff
  the tmux session is alive. Find the session with
  `tmux has-session -t <tmux_session>`.
- `worktree`, `branch`, `cwd`
- `started`, `last_touched`, `shutdown` — timestamps, format flexible
- `resumed_session_id` — Claude `--resume` token for the primary
  worker (window 0 pane 0), captured at shutdown or wrap. Always
  written and surfaced in full — never truncated or abbreviated with
  `<prefix>-...`. Reading the registry alone should be enough to fire
  a manual `claude --resume` for the common single-worker case.
- `snapshot` — path to multi-window pane snapshot captured at shutdown
  or wrap
- `resume_state` — path to the structured per-window state file under
  `~/.local/state/claude-manager/resume/` written at shutdown. See
  the Shutdown section for format.
- `resume_target` — expected resume date (optional, free-form)
- `wrap_requested` — `true` when a worker has requested wrap; the
  manager fulfils the journal-write phase and removes the entry
- `notes` — string OR a path to a file (typically a journal entry)

Lifecycle state is encoded by which fields are present:

- `tmux_session` set → active.
- No `tmux_session`, `shutdown` + `resumed_session_id` + `resume_state`
  set → shutdown.
- No `tmux_session`, `wrap_requested: true` set → wrap in progress
  (worker has marked, manager hasn't fulfilled).

There is no explicit status field — derived from the field set.

Reads/writes are full-file. Use a `mkdir` lock for mutual exclusion
(cross-platform; `flock` is Linux-only). Hold the lock only around the
rewrite — not across snapshot capture or tmux moves:

```bash
_reg="$HOME/.local/state/claude-manager/sessions.md"
_lock="${_reg}.lock"
while ! mkdir "$_lock" 2>/dev/null; do sleep 0.1; done
# read $_reg, mutate, write $_reg (Edit tool is fine)
rmdir "$_lock"
```

Each Bash tool call is a fresh shell — `trap`-based release does not
survive across calls, so don't rely on it. Release explicitly. If a
flow aborts with the lock held, recover with
`rmdir ~/.local/state/claude-manager/sessions.md.lock`.

Preserve unknown fields, header lines, and stray prose on rewrite.

Example:

```markdown
# Sessions

manager: 0:1.0

## eng-1234-payment-bug
ticket: ENG-1234
tmux_session: eng-1234-payment-bug
worktree: ~/code/repo-foo/_wt/eng-1234
branch: fix/eng-1234-payment-bug
started: 2026-04-29 14:00
last_touched: 2026-04-29 16:20
notes: ~/code/journal/2026-04-29-eng-1234.md
```

(Active; the tmux session named `eng-1234-payment-bug` is alive.
A shutdown entry would drop `tmux_session` and add `shutdown`,
`snapshot`, `resume_state`, `resumed_session_id`.)

## Registry as shared channel

The registry is shared state between the manager and its workers. The
mkdir lock serialises writes from either side.

**Workers may write to their own entry only.** Allowed fields:
`last_touched`, `notes`, ticket/branch updates, and the shutdown/wrap
fields (`shutdown`, `resumed_session_id`, `snapshot`, `resume_state`,
`wrap_requested`). Workers may also write their own snapshot file
under `~/.local/state/claude-manager/snapshots/` and resume_state
file under `~/.local/state/claude-manager/resume/`.

**Workers must not touch:** the header block, any other session's
entry, or the journal/wiki. Wrap is driven via the `wrap_requested`
marker; the manager does the journal write.

**Manager continues to own:** header refresh for itself; the spawn
flow; journal entries and other knowledge work; reconciling against
`tmux ls`.

Both manager-initiated lifecycle transitions (driven by user requests
to the manager directly) and worker-initiated transitions (via the
`/claude-manager-shutdown` and `/claude-manager-wrap` skills) end up
in the same registry state. The two paths are described in the
Shutdown and Wrap sections.

## Watching the registry

The manager runs a background watch process so worker changes show up
in the manager's task list live, not just on the manager's next turn.

Watch command (portable):

```bash
registry=~/.local/state/claude-manager/sessions.md
last=$(stat -f %m "$registry" 2>/dev/null || stat -c %Y "$registry")
while sleep 1; do
  cur=$(stat -f %m "$registry" 2>/dev/null || stat -c %Y "$registry")
  if [ "$cur" != "$last" ]; then
    echo "changed:$cur"
    last=$cur
  fi
done
```

Spawn it via `Bash` with `run_in_background: true` and consume its
stdout via `Monitor`. Each `changed:` line is a notification — the
manager reacts between turns.

Lifecycle:

- **PID file** at `~/.local/state/claude-manager/watch.<sanitised-mgr-pane>.pid`
  — keyed by the manager's tmux address with `:` and `.` replaced by
  `-` (`tr ':.' '--'`) so the basename has a single `.pid` extension.
  On invocation: if the PID file exists and the PID is alive, reuse
  it; else start a new watch and write the PID.
- **Reaction loop:** on each `changed:` event, re-read the registry,
  diff against the in-conversation last-known state, surface a brief
  note for any worker-driven change ("worker `eng-1234` shut itself
  down"), and update the task list per the hygiene rule.
- **Self-writes don't double-fire.** When the manager writes to the
  registry it updates its in-conversation last-known state *before*
  releasing the lock. The watch event arrives next turn and re-reads
  a registry already matching the in-conversation state, so the diff
  is empty and nothing is surfaced.
- **`wrap_requested: true`** is a special diff: trigger the
  manager-side wrap-fulfilment phase (journal write, registry
  removal — see Wrap).

Fallback: if the watch process is missing on a registry-touching
action (it died, or this is a fresh `claude --resume`), the manager
re-stats the registry on the spot, surfaces any drift, then restarts
the watch. The watch is the fast path; explicit re-stat is the safety
net.

## Spawning a session

1. Clarify ticket, worktree, branch — only what matters.
2. If a worktree is wanted, create it first. Worktrees must exist
   before Claude starts inside them: `cwd` cannot change later. Use
   whatever the project rules say.
3. Pre-check name collision:

   ```bash
   tmux has-session -t "$session_id" 2>/dev/null
   ```

   If a tmux session by that name already exists, surface it and ask:
   import (see Importing an existing tmux session) or pick a new
   session id. Don't silently take over.
4. Create the tmux session and capture its name:

   ```bash
   tmux new-session -d -s "$session_id" -n "$session_id" -c "$cwd"
   ```

   `-d` keeps the focus rule (no stealing the user's view). `-n`
   names window 0 after the session id for tidiness; the user is free
   to rename later. Additional windows or panes inside this session
   are user free space — the registry doesn't track them while alive.
5. Decide the brief — what the manager types into the worker's input
   box on the first prompt. Default is narrow: one line stating the
   working directory and the topic the user named, optionally one
   line of obvious context (existing branch, open todos, files to
   start with). Hand back to the user for direction inside the
   session.

   Do NOT include: multi-step plan, list of candidate scopes,
   reproductions, prescribed commits, implicit time pressure, or
   directive phrasings the worker will fixate on ("read-only, hand
   back after", "ONLY do X"). Those are manager work disguised as a
   brief; they bias the worker before they've read the room.

   If I have observations worth surfacing, list them to the user in
   chat *before* spawning. They can fold them into the brief, ignore,
   or defer.

   Exceptions where a fuller brief is fine:
   - The user explicitly described the work in their message.
   - The worker is for research on a ticket the user pinged.
   - The user said "spawn a worker that does X" rather than "open a
     session for me to work on X".

   If the user asks for a "blank" or "empty" spawn, send no brief at
   all — they'll type the first prompt themselves.
6. Start Claude in window 0 pane 0 (the primary worker). Kickoff:
   launch `claude`, poll the pane until the TUI input line is ready,
   send the brief from step 5 (skip the send-keys when the brief is
   blank), then `Enter` as separate `send-keys` calls. Capture the
   pane afterwards to confirm the prompt left the input box. The
   polling check must match what the current TUI renders — capture
   first and adapt the regex.

   ```bash
   target="${session_id}:0.0"
   tmux send-keys -t "$target" "claude" Enter
   # Example shape — adapt the check to whatever this TUI renders:
   until tmux capture-pane -p -t "$target" -S -5 | grep -q '<marker>'; do
     sleep 0.5
   done
   if [ -n "$brief" ]; then
     tmux send-keys -t "$target" "$brief"
     tmux send-keys -t "$target" Enter
   fi
   tmux capture-pane -p -t "$target" -S -5
   ```
7. Add the session to the registry with `tmux_session: $session_id`.
   Add to the visible task list (`[active]` prefix).
8. Tell the user how to switch to it (see Switch UX).

All session-creating tmux commands include `-d` so spawning sessions
never steals the user's focus. Hard rule.

For **PR review** tasks specifically, the worktree is off the PR's
branch (`gh pr view <N> --json headRefName`). The worker's `review-pr`
skill takes over once the worker starts; the manager just lands it in
the right worktree.

## Switch UX

Primary: `prefix+w` picker — interactive list across sessions and
windows. Direct: `tmux switch-client -t <session-id>`. Manager hands
back the session id; the user navigates.

## Reconcile

On demand, diff the registry against `tmux ls`:

- Entry with `tmux_session` set: `tmux has-session -t <tmux_session>`.
  If alive, fine. If not alive, the session died unexpectedly —
  surface and ask: finished, shutdown unexpectedly, or unknown?

  If *many* entries fail lookup at once, suspect a tmux server
  restart and surface aggregately before changing any state. Unlike
  numeric window ids, `tmux_session` is a user-namespace name and
  cannot be recycled into pointing at something unrelated — the
  worst case is everything missing at once.

- Entry without `tmux_session`: check for `shutdown` (leave alone) or
  `wrap_requested: true` (trigger manager-side wrap if the watch
  missed it). Otherwise ask the user.

- tmux session present but no matching registry entry → ask: import
  or ignore. Don't silently take ownership.

Sync the visible task list after reconciling.

The watch auto-triggers a reconcile pass on any worker write, so
manual reconcile is mostly only needed for tmux-side drift the watch
can't see.

## Importing an existing tmux session

1. `tmux has-session -t <existing-name>` to confirm the session
   exists.
2. If the existing tmux session name equals the desired registry
   session id, register as-is. If different, either rename the tmux
   session (`tmux rename-session`) or adopt the existing name as the
   registry session id.
3. Ask for missing context (ticket, branch, worktree, anything else
   worth recording).
4. Write the registry entry with `tmux_session: <name>`, `started`,
   `last_touched`. Add to the visible task list (`[active]` prefix).
5. Hand over the switch handle (see Switch UX).

## Shutdown

Shutdown = kill the tmux session; keep the registry entry so the
session can be cold-resumed later via the manager. Sits between
active and Wrap (final, journal entry written, entry removed).
Flexible wording: "shutdown", "kill that one", "pause it", "drop
tmux".

**Mechanics:**

1. **Discover structure.** Walk every window and every pane in the
   tmux session, capturing window layouts and per-pane metadata:

   ```bash
   tmux list-windows -t "$tmux_session" \
     -F '#{window_index} #{window_name} #{window_layout}'
   # per window:
   tmux list-panes -t "$tmux_session":<w> \
     -F '#{pane_index} #{pane_current_path} #{pane_current_command}'
   ```

2. **Capture pane snapshots.** Concatenate every pane in every
   window into one snapshot file, with `--- window <w> pane <p> ---`
   markers:

   ```bash
   snapshot=~/.local/state/claude-manager/snapshots/<session-id>.txt
   mkdir -p "$(dirname "$snapshot")"
   tmux list-windows -t "$tmux_session" -F '#{window_index}' | while read w; do
     tmux list-panes -t "$tmux_session":$w -F '#{pane_index}' | while read p; do
       echo "--- window $w pane $p ---"
       tmux capture-pane -p -J -t "${tmux_session}:${w}.${p}" -S -500
       echo
     done
   done > "$snapshot"
   ```

3. **Find Claude session IDs for every Claude pane.** Walk every pane
   from step 1. A pane is Claude if `pane_current_command` contains
   `claude`, OR a capture of its last ~30 lines
   (`tmux capture-pane -p -J -t <pane> -S -30`) contains
   `esc to interrupt` or ends with a trailing `> ` prompt. For each:

   - If this is the primary pane (window 0 pane 0) and the registry
     entry already has `resumed_session_id`, reuse it — `claude
     --resume <id>` continues writing to the same JSONL, so the id is
     stable across resume cycles.

   - Otherwise, Claude stores per-project JSONL conversation files
     under `~/.claude/projects/<encoded-cwd>/`. The encoding converts
     every `/`, `.` and `_` in the absolute cwd to `-`; don't strip
     the leading `/` (it produces a leading `-` on the directory
     name, which is correct — e.g.
     `/Users/foo.bar/code/my_service` →
     `-Users-foo-bar-code-my-service`):

     ```bash
     cwd="<pane_current_path>"
     encoded=$(echo "$cwd" | sed 's|[/._]|-|g')
     proj_dir="$HOME/.claude/projects/$encoded"
     ls -t "$proj_dir"/*.jsonl 2>/dev/null
     ```

     Identify the JSONL by grepping the snapshot for a distinctive
     phrase (initial prompt, early output) and matching against the
     candidate JSONL files. The basename without `.jsonl` is the
     `claude_session_id` for that pane.

   - **Shared project dirs** (multiple Claude panes with the same
     cwd): each pane's section of the snapshot contains its
     distinctive initial prompt. If no phrase is unique enough, fall
     back to mtime — most recently modified JSONL not already claimed
     by another pane. Note the method used in the resume_state file.

   - Do **not** use `lsof` on the tmux pane's PID to locate the JSONL
     — on macOS `lsof` does not expose it.

4. **Build the resume_state file** at
   `~/.local/state/claude-manager/resume/<session-id>.md`. Markdown,
   same idiom as the registry. One window per `## window <n>: <name>`
   block with a `layout:` field; one pane per `### pane <n>` sub-block
   with `cwd:`, `command:`, and on Claude panes `claude_session_id:`.
   For panes whose `pane_current_command` is just a shell (`bash`,
   `zsh`, `fish`), set `command:` empty — auto-replaying an idle
   shell on resume is noise. Example:

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

5. **Acquire the lock, rewrite the registry entry, release the lock**
   (see Registry section). The rewrite:
   - Adds `resumed_session_id` (primary worker = window 0 pane 0).
   - Adds `snapshot: <path>`, `resume_state: <path>`,
     `shutdown: <today>`.
   - Adds `resume_target: <date>` if known.
   - Updates `last_touched`.
   - Appends to `notes`: "Tmux killed <date>; resume via the manager."
   - Removes `tmux_session`.

6. **Kill the tmux session** (after the lock is released):

   ```bash
   tmux kill-session -t "$tmux_session"
   ```

7. **Update the visible task list:** set the description prefix to
   `[shutdown]` per the Task list hygiene rule.

**Trigger paths:**

- The user asks the manager directly. Manager runs the full mechanics
  above.
- A worker self-shuts via `/claude-manager-shutdown`. The worker does
  steps 1–6 itself. For multi-Claude sessions (forked workers in other
  windows), the worker walks all panes per step 3, not just its own.
  The manager observes the change via the watch and updates the task
  list.

**To resume later:** the manager's cold-resume flow rebuilds the
whole tmux session from the resume_state file (see Cold resume
below). A manual `claude --resume <resumed_session_id>` from the
worktree still works as an escape hatch for the primary worker only.

## Cold resume

Cold resume = bring a shutdown session back from disk. Manager-only
operation; workers can't cold-resume their own dead session (the
worker doesn't exist yet — cold resume is what creates it).

**Mechanics:**

1. Read the registry shutdown entry plus the `resume_state` file at
   the path the entry references.
2. Pre-check name collision: `tmux has-session -t "$session_id"`
   must return non-zero. Fail loud if alive — there's another tmux
   session by that name; ask the user before overwriting.
3. Recreate the session window-by-window. For the first window:

   ```bash
   tmux new-session -d -s "$session_id" \
     -n "$window0_name" -c "$pane0_cwd"
   ```

   For each subsequent window in the resume_state file, in order:

   ```bash
   tmux new-window -d -t "$session_id": -n "$name" -c "$pane0_cwd"
   ```

4. For each window, add splits for each pane index > 0 recorded in
   the resume_state file:

   ```bash
   tmux split-window -d -t "$session_id":<w> -c "$pane_n_cwd"
   ```

5. Apply the captured layout to restore the geometry:

   ```bash
   tmux select-layout -t "$session_id":<w> "$layout"
   ```

   This must come after the splits (the layout assumes a specific
   pane count); before sending commands (we want the right panes
   running the right commands).
6. Send the recorded command per pane, skipping empty ones (idle
   shells stay idle):

   ```bash
   tmux send-keys -t "$session_id":<w>.<p> "$command" Enter
   ```

   Claude panes get their `claude --resume <claude_session_id>` line
   verbatim. Other panes get their recorded command.
7. Update the registry under lock:
   - Add `tmux_session: $session_id`.
   - Remove `shutdown`, `snapshot`, `resume_state`,
     `resumed_session_id`.
   - Update `last_touched`.

   Delete the on-disk snapshot and resume_state files — the live
   session supersedes them.
8. Update the visible task list: prefix `[shutdown]` → `[active]`.

**Manual escape hatch.** A user can always run `claude --resume <id>`
from the worktree to bring the primary worker back without the
surrounding scaffold. That route doesn't update the registry; the
next reconcile catches the drift.

## Wrap

Wrap = final close-out. Journal entry written per the project's
schema; registry entry removed; tmux session killed. Flexible
wording: "wrap up", "complete", "close out", "finish".

**Mechanics:**

1. Capture `tmux_session` into a shell var — the registry-removal
   step below wipes it, so the kill needs it remembered.
2. Capture pane snapshots for every pane in every window into one
   snapshot file, with `--- window <w> pane <p> ---` markers (same
   format as shutdown):

   ```bash
   snapshot=~/.local/state/claude-manager/snapshots/<session-id>.txt
   mkdir -p "$(dirname "$snapshot")"
   tmux list-windows -t "$tmux_session" -F '#{window_index}' | while read w; do
     tmux list-panes -t "$tmux_session":$w -F '#{pane_index}' | while read p; do
       echo "--- window $w pane $p ---"
       tmux capture-pane -p -J -t "${tmux_session}:${w}.${p}" -S -200
       echo
     done
   done > "$snapshot"
   ```

3. Write the journal entry per the project's schema, using the
   snapshot and any `notes` from the registry entry. If notes are
   thin and there's no obvious narrative from snapshot + recent git
   activity in the worktree, ask the user a focused question before
   writing. Otherwise proceed.
4. Mark the visible task list entry `completed`, then delete it from
   the list.
5. Remove the entry from the registry.
6. Kill the tmux session if it's still alive:

   ```bash
   tmux kill-session -t "$tmux_session"
   ```

**Trigger paths:**

- The user asks the manager directly. Manager runs the full
  mechanics above.
- A worker self-wraps via `/claude-manager-wrap`. The worker captures
  the snapshot, resolves its `resumed_session_id`, gathers any
  context for the journal into `notes`, sets `wrap_requested: true`
  on its registry entry, then kills the tmux session. The watch
  fires; the manager picks up the marker and runs steps 3–5
  (journal write, task-list completion, registry removal). Step 6
  (kill) is already done.

If the manager isn't running when a worker wraps, the
`wrap_requested` marker persists on the entry. The next manager
invocation sees it during reconcile and processes it.

The split (worker pre-captures and kills; manager writes the journal
and removes the entry) is preserved from the previous design. Shifting
to a marker-only flow is tracked as a separate followup.

## Knowledge work

The manager owns the project's recordkeeping.

When the project rulebook (already in Claude's context on startup)
points to a journal, wiki, investigations dir or similar, write into
them at:

- Session wrap (the snapshot from the wrap flow goes here).
- Mid-session decisions, conventions or lessons worth outliving the
  worker.
- On request.

Read the relevant store's schema before writing. Follow it.

## Idle-detection query

"Which workers are waiting for input?" — for each registry session
with `tmux_session` set, walk every pane in the session and capture
the recent tail:

```bash
tmux list-panes -s -t <tmux_session> \
  -F '#{window_index} #{pane_index} #{pane_current_command}' \
  | while read w p cmd; do
    [ -n "$cmd" ] || continue
    tmux capture-pane -p -t "<tmux_session>:${w}.${p}" -S -30
  done
```

Filter for Claude panes: `pane_current_command` contains `claude`, OR
the captured tail (last ~30 lines via
`tmux capture-pane -p -J -t <pane> -S -30`) contains
`esc to interrupt` or ends with a trailing `> ` prompt.

Heuristics (Claude Code TUI):

- **Idle**: trailing `> ` prompt; no `esc to interrupt`; no spinner.
- **Busy**: `esc to interrupt` present; spinner; streaming output.
- **Not running Claude**: shell prompt or other tool signature.

Heuristic, not authoritative. Report best-effort with evidence.

Other queries (switch back, what's around) are direct registry
reads/writes — update `last_touched` and the visible task list as
needed.

## Resource awareness

If workers report claimed ports, dev servers etc, capture under the
session's notes. Warn on plausible conflicts when spawning a new
session. No automatic scanning.

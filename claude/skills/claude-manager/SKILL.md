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
fields (`shutdown`, `resumed_session_id`, `snapshot`, `wrap_requested`).
Workers may also write to their own snapshot file under
`~/.local/state/claude-manager/snapshots/`.

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
5. Start Claude in window 0 pane 0 (the primary worker). Kickoff:
   launch `claude`, poll the pane until the TUI input line is ready,
   then send the prompt text and `Enter` as separate `send-keys`
   calls. Capture the pane afterwards to confirm the prompt left the
   input box. The polling check must match what the current TUI
   renders — capture first and adapt the regex.

   ```bash
   target="${session_id}:0.0"
   tmux send-keys -t "$target" "claude" Enter
   # Example shape — adapt the check to whatever this TUI renders:
   until tmux capture-pane -p -t "$target" -S -5 | grep -q '<marker>'; do
     sleep 0.5
   done
   tmux send-keys -t "$target" "$prompt"
   tmux send-keys -t "$target" Enter
   tmux capture-pane -p -t "$target" -S -5
   ```
6. Add the session to the registry with `tmux_session: $session_id`.
   Add to the visible task list (`[active]` prefix).
7. Tell the user how to switch to it (see Switch UX).

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

Shutdown = kill the tmux container; keep the registry entry for later
resumption via `claude --resume <id>`. Sits between Park (keeps tmux
alive) and Wrap (final, journal entry written, entry removed).
Flexible wording: "shutdown", "kill that one", "pause it", "drop
tmux".

**Mechanics:**

1. **Capture pane snapshots.** Capture every pane in the tmux
   container, not just `claude_panes` — other panes may hold dev
   servers, shells, or other context worth preserving:

   ```bash
   tmux list-panes -t <target> -F '#{pane_index}' | while read p; do
     echo "--- pane $p ---"
     tmux capture-pane -p -J -t <target>.$p -S -500
     echo
   done >> ~/.local/state/claude-manager/snapshots/<session-id>.txt
   ```

2. **Find the Claude session ID.** If the registry entry already has
   `resumed_session_id`, reuse it — `claude --resume <id>` continues
   writing to the same JSONL, so the id is stable across resume
   cycles. Skip the rest of this step.

   Otherwise, Claude stores per-project JSONL conversation files
   under `~/.claude/projects/<encoded-cwd>/`. The encoding converts
   every `/`, `.` and `_` in the absolute cwd to `-`; don't strip the
   leading `/` (it produces a leading `-` on the directory name,
   which is correct — e.g.
   `/Users/foo.bar/code/my_service` →
   `-Users-foo-bar-code-my-service`):

   ```bash
   cwd="<worktree or cwd from registry>"
   encoded=$(echo "$cwd" | sed 's|[/._]|-|g')
   proj_dir="$HOME/.claude/projects/$encoded"
   ls -t "$proj_dir"/*.jsonl 2>/dev/null
   ```

   To identify which JSONL belongs to this session, grep the snapshot
   for a distinctive phrase (initial prompt, early output) and match
   against the candidate JSONL files. The basename without `.jsonl`
   is the `resumed_session_id`.

   **Shared project dirs** (multiple sessions in the same cwd, e.g.
   three separate my_service workers): each session's snapshot will
   contain its distinctive initial prompt. If no phrase is unique
   enough, fall back to mtime — most recently modified JSONL not
   already claimed by another registry entry. Note the method used
   in `notes`.

   Do **not** use `lsof` on the tmux pane's PID to locate the JSONL
   — on macOS `lsof` does not expose it.

3. **Capture `tmux_window_id` into a shell var before rewriting** —
   the registry write below removes it from the entry, so the kill
   step needs it remembered:

   ```bash
   wid="$(read it from the registry entry)"
   ```

4. **Acquire the lock, rewrite the entry, release the lock** (see
   Registry section). The rewrite:
   - Adds `resumed_session_id`, `snapshot`, `shutdown: <today>`.
   - Adds `resume_target: <date>` if known.
   - Updates `last_touched`.
   - Appends to `notes`: "Tmux killed <date>; resume via
     `claude --resume <id>` from the worktree/cwd."
   - Removes `tmux_window_id`, `tmux_session`, `claude_panes`.

5. **Kill the tmux container** (after the lock is released). Pass the
   stable id directly — `tmux kill-window` accepts `@N` targets, no
   resolve-then-kill round trip needed (which would be racy):

   ```bash
   tmux kill-window -t "$wid"
   ```

   If the parent tmux session was a standalone one (parked) and that
   was its only window, it dies with the window — no extra step.
   No renumber afterwards; gappy indices are fine.

6. **Update the visible task list:** set the description prefix to
   `[shutdown]` per the Task list hygiene rule.

**Trigger paths:**

- The user asks the manager directly. Manager runs the full mechanics
  above.
- A worker self-shuts via `/claude-manager-shutdown`. The worker does
  steps 1–5 itself (it has direct access to its own JSONL — most
  recently modified is by definition this session). The manager
  observes the change via the watch and updates the task list.

**To resume later:** from the worktree (or `cwd`):

```bash
claude --resume <resumed_session_id>
```

The snapshot provides context on where the session left off.

## Wrap

Wrap = final close-out. Journal entry written per the project's
schema; registry entry removed; tmux container killed. Flexible
wording: "wrap up", "complete", "close out", "finish".

**Mechanics:**

1. Capture `tmux_window_id` (if present on the entry) into a shell
   var — the registry-removal step below wipes it, so the kill needs
   it remembered:
   `wid="$(read it from the registry entry)"`.
2. For each pane in `claude_panes` (or every pane, when killing the
   container), capture
   `tmux capture-pane -p -t <target>.<pane> -S -200` for a final
   snapshot.
3. Write the journal entry per the project's schema, using the
   snapshot and any `notes` from the registry entry. If notes are
   thin and there's no obvious narrative from snapshot + recent git
   activity in the worktree, ask the user a focused question before
   writing. Otherwise proceed.
4. Mark the visible task list entry `completed`, then delete it from
   the list.
5. Remove the entry from the registry.
6. Kill the tmux container if `wid` was set: `tmux kill-window -t
   "$wid"`. Pass the stable id directly to avoid the lookup-then-kill
   race. No renumber afterwards.

**Trigger paths:**

- The user asks the manager directly. Manager runs the full
  mechanics above.
- A worker self-wraps via `/claude-manager-wrap`. The worker captures
  the snapshot, resolves its `resumed_session_id`, gathers any
  context for the journal into `notes`, sets `wrap_requested: true`
  on its registry entry, then kills the tmux container. The watch
  fires; the manager picks up the marker and runs steps 2–4
  (journal write, task-list completion, registry removal). Step 5
  (kill) is already done.

If the manager isn't running when a worker wraps, the
`wrap_requested` marker persists on the entry. The next manager
invocation sees it during reconcile and processes it.

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

Filter for Claude panes by `pane_current_command` containing "claude"
or by sniffing the captured tail for TUI signatures.

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

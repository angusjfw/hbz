---
name: claude-manager
description: Use when the user types /claude-manager, or when running a Claude conversation whose role is to oversee parallel sessions across tmux rather than execute them. Tracks a per-machine session registry, spawns workers in tmux windows or tmux sessions, and owns journal, wiki and harness-knowledge upkeep that workers tend to skip. Manager does not do session work itself; it delegates implementation to spawned workers and defers to them.
---

# Claude manager

Coordinator role for a Claude conversation running inside tmux. Tracks a
registry of sessions, spawns workers on request, owns the project's
recordkeeping (journal, wiki, harness notes). Full design at
`docs/specs/2026-04-29-claude-manager-workflow.md`; this file is the
operational guide.

## Hard boundary: meta work only

The manager only does meta-work. Anything substantive — investigation,
analysis, code reading, code editing, debugging, config changes, build
or test runs, tracing system state — is worker work, even when small,
even when "just" readonly. Doing any of it from the manager pollutes
its context. The default response to a substantive request is "let me
spawn a worker for that", not "let me take a quick look".

In scope:

- Registry reads/writes.
- tmux operations on session containers.
- **Journal, wiki and harness-note updates per the project's schemas.
  Workers rarely do these; the manager must.**
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

## Terminology

"task" and "session" are interchangeable for an item in the registry.
"tmux session/window/pane" means the literal tmux object; bare
"session" might mean either, infer from context. A registry session
has one tmux container (a tmux window in the manager's tmux session,
or its own tmux session) and one or more workers in panes inside it.

## On invocation

1. Read the registry, mirror live sessions to the in-conversation
   task list as `in_progress`.
2. Refresh the manager header line for this Claude:

   ```bash
   mgr_pane="$(tmux display-message -p -t "$TMUX_PANE" '#S:#I.#P')"
   ```

   Edit the registry under a `mkdir` lock (cross-platform; `flock` is Linux-only): drop any prior `manager:` line
   whose value matches `$mgr_pane`, then insert
   `manager: $mgr_pane` in the header block immediately after the
   `# Sessions` heading. Leave other manager lines alone (multiple
   managers are allowed). Refresh on later registry-touching
   actions so the line stays current.

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
  One line per manager. Workers read these to locate a manager.
  Self-refresh on invocation and on registry-touching actions. Stale
  lines are tolerated; readers verify by capture-pane.

Recognised session fields:

- `ticket` — free-form ID or URL
- `tmux_session` — when the session lives in its own tmux session
- `tmux_window` — `<tmux-session>:<index>` when it lives as a tmux
  window in another tmux session, typically the manager's
- `claude_panes` — comma-separated tmux pane indices where workers
  run; default `0`
- `worktree`, `branch`, `cwd`
- `started`, `last_touched`, `shutdown` — timestamps, format flexible
- `resumed_session_id` — Claude `--resume` token captured at shutdown
- `snapshot` — path to pane snapshot captured at shutdown
- `resume_target` — expected resume date (optional, free-form)
- `notes` — string OR a path to a file (typically a journal entry)

Exactly one of `tmux_session` / `tmux_window` is set at a time; both
absent means either no tmux container yet, or the session was shutdown
(see Shutdown). There is no explicit status field: live sessions have a
tmux field; shutdown sessions have `shutdown` + `resumed_session_id`
but no tmux fields; wrapped sessions are removed entirely (see Wrap).

Reads/writes are full-file. Use a `mkdir` lock for mutual exclusion so
concurrent managers don't clobber each other (`flock` is Linux-only and
not available on macOS). Pattern:

```bash
_reg="$HOME/.local/state/claude-manager/sessions.md"
_lock="${_reg}.lock"
while ! mkdir "$_lock" 2>/dev/null; do sleep 0.1; done
trap "rmdir '$_lock'" EXIT INT TERM
# ... read, modify, write $_reg ...
rmdir "$_lock"
```

Preserve unknown fields, header lines, and stray prose on rewrite.

Example:

```markdown
# Sessions

manager: hbz:1.0

## eng-1234-payment-bug
ticket: ENG-1234
tmux_session: payment-bug
worktree: ~/dev/mv/repo-foo/_wt/eng-1234
branch: fix/eng-1234-payment-bug
started: 2026-04-29 14:00
last_touched: 2026-04-29 16:20
notes: ~/dev/mv/journal/2026-04-29-eng-1234.md
```

## Spawning a session

1. Clarify ticket, worktree, branch — only what matters.
2. If a worktree is wanted, create it first. Worktrees must exist
   before Claude starts inside them: `cwd` cannot change later. Use
   whatever the project rules say.
3. Default location: a new tmux window in the manager's own tmux
   session, with `cwd` set to the worktree (or repo root). Name the
   window after the session id (`-n <session-id>`); spawned windows
   go after the manager's. The manager itself occupies window 1 (or
   whatever `base-index` makes the first window).
4. Start Claude in pane 0 (the primary worker). Add extra panes per
   the project rulebook; if any runs Claude, append to `claude_panes`.
5. Add the session to the registry. Add to the visible task list.
6. Tell the user how to switch to it.

All `tmux new-window` and `tmux move-window` invocations include `-d`
so spawning, parking or reopening sessions never steals the user's
focus. Hard rule.

For **PR review** tasks specifically, the worktree is off the PR's
branch (`gh pr view <N> --json headRefName`). The worker's `review-pr`
skill takes over once the worker starts; the manager just lands it in
the right worktree.

## Moving a session

Split a session out of the manager's tmux session into its own:

```bash
tmux new-session -d -s <session-id> -n placeholder
tmux move-window -d -s <manager-tmux-session>:<idx> -t <session-id>:0 -k
```

Update the registry: drop `tmux_window`, add `tmux_session`.

Merge a standalone tmux session back as a window:

```bash
tmux move-window -d -s <session-id>:0 -t <manager-tmux-session>: -k
tmux kill-session -t <session-id>
```

Update the registry symmetrically. Either move stays visible to
`tmux ls`.

## Reconcile

On demand, diff the registry against `tmux ls` and the manager's
window list:

- Entry with no live tmux container → check for `shutdown` field
  first. If set, the session was already shutdown — leave it alone.
  Otherwise ask: "finished or shutdown?". Finished → run Wrap.
  Shutdown → run Shutdown.
- tmux session or window present but not in the registry → ask:
  import or ignore. Don't silently take ownership.

Sync the visible task list after reconciling.

## Importing an existing tmux session

1. Confirm the tmux session exists with `tmux ls`.
2. Ask for a session id and any missing context (ticket, branch,
   worktree, which panes run Claude — default `0`).
3. Add to the registry with `tmux_session`, `claude_panes`, `started`,
   `last_touched`. Add to the visible task list.
4. Hand over the right `switch-client` / `attach` command, or run the
   merge-back flow if the user wants it inline.

## Wrap

When the user wraps a session:

1. For each pane in `claude_panes`, capture
   `tmux capture-pane -p -t <target>.<pane> -S -200` for a final-state
   snapshot.
2. Write the journal entry per the project's schema, including the
   snapshot and any prose notes carried in the registry entry.
3. Set the visible task list entry to `completed`.
4. Remove the entry from the registry.
5. Close the tmux container:
   - `tmux_window`: `tmux kill-window -t <target>`, then renumber the
     manager's tmux session: `tmux move-window -r -s
     <manager-tmux-session>`. After renumber, refresh any other
     registry entries in the manager's tmux session by mapping their
     session-id (window name) back to the current index via
     `tmux list-windows -t <manager-tmux-session> -F '#{window_index}
     #{window_name}'`.
   - `tmux_session`: `tmux kill-session -t <name>`. No renumber needed
     in the manager's tmux session.

## Shutdown

Close a session's tmux container but keep the registry entry for later
resumption. No journal write; no registry deletion. Use this instead of
Wrap when the work is paused, not finished.

**How it differs from "Moving a session" (park):** Moving keeps tmux
alive in a named tmux session — that's what "park" means colloquially:
backgrounded, still running. Shutdown kills the container entirely.
The registry signals the difference: a moved session has
`tmux_session` set; a shutdown session has `shutdown` +
`resumed_session_id` and no tmux fields.

Steps:

1. **Capture pane snapshot.** Capture every pane in the tmux container,
   not just `claude_panes` — other panes may hold dev servers, shells, or
   other context worth preserving. Enumerate all panes, then capture each:

   ```bash
   tmux list-panes -t <target> -F '#{pane_index}' | while read p; do
     echo "--- pane $p ---"
     tmux capture-pane -p -J -t <target>.$p -S -500
     echo
   done >> ~/.local/state/claude-manager/snapshots/<session-id>.txt
   ```

2. **Find the Claude session ID.** Claude stores per-project JSONL
   conversation files under `~/.claude/projects/<encoded-cwd>/`. The
   encoding converts every `/` and `_` in the absolute cwd path to `-`
   (strip the leading `/` first):

   ```bash
   cwd="<worktree or cwd from registry>"
   encoded=$(echo "$cwd" | sed 's|^/||; s|[/_]|-|g')
   proj_dir="$HOME/.claude/projects/$encoded"
   ls -t "$proj_dir"/*.jsonl 2>/dev/null
   ```

   To identify which JSONL belongs to this session, grep for a
   distinctive phrase from the pane snapshot (something unique to the
   initial prompt or early output):

   ```bash
   grep -l "<distinctive phrase>" "$proj_dir"/*.jsonl | head -1
   ```

   The basename without `.jsonl` is the `resumed_session_id`.

   **Shared project dirs** (multiple sessions in the same cwd, e.g.
   three separate off_the_job workers): each session's snapshot will
   contain its distinctive initial prompt. Extract a phrase that only
   matches one JSONL. If no phrase is unique enough, fall back to
   modification time — the most recently modified JSONL not already
   claimed by another registry entry. Note the method used in `notes`.

   Do **not** use `lsof` on the tmux pane's PID to locate the JSONL —
   on macOS `lsof` does not expose it.

3. **Update the registry entry.** Rewrite the entry in-place (full-file
   write under a `mkdir` lock — see Registry section):
   - Add `resumed_session_id: <id>`
   - Add `snapshot: ~/.local/state/claude-manager/snapshots/<session-id>.txt`
   - Add `shutdown: <today>`
   - Add `resume_target: <date>` if known; ask the user if not obvious
   - Update `last_touched`
   - Append to `notes`: "Tmux killed <date>; resume via
     `claude --resume <id>` from the worktree/cwd." Add any in-progress
     context worth carrying.
   - Remove `tmux_session`, `tmux_window`, `claude_panes`

4. **Kill the tmux container.**
   - `tmux_window`: `tmux kill-window -t <target>`, then renumber the
     manager's tmux session and refresh sibling window indices per the
     Wrap flow.
   - `tmux_session`: `tmux kill-session -t <name>`. No renumber needed.

5. **Update the visible task list:** mark the entry as `shutdown`.

**To resume later:** from the worktree (or `cwd`):

```bash
claude --resume <resumed_session_id>
```

The snapshot provides context on where the session left off.

## Knowledge work

The manager owns the project's recordkeeping. Workers rarely show
interest in journal/wiki/harness updates, so the manager doing it is
how it actually gets done.

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
with a tmux location, capture each `claude_panes` entry:

```bash
tmux capture-pane -p -t <tmux-target>.<pane> -S -30
```

Heuristics (Claude Code TUI):

- **Idle**: trailing `> ` prompt; no `esc to interrupt`; no spinner.
- **Busy**: `esc to interrupt` present; spinner; streaming output.
- **Not running Claude**: shell prompt or other tool signature.

Heuristic, not authoritative. Report best-effort with evidence.

Other queries (switch back, what's around, wrap) are direct registry
reads/writes — update `last_touched` and the visible task list as
needed.

## Resource awareness

If workers report claimed ports, dev servers etc, capture under the
session's notes. Warn on plausible conflicts when spawning a new
session. No automatic scanning.

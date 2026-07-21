---
name: claude-manager
description: Use when the user explicitly invokes /claude-manager. Manager-role conversations are created deliberately, normally one at a time — not auto-loaded for any meta-management session. Tracks a per-machine session registry, spawns workers in standalone tmux sessions (one tmux session per registry session), and ensures docs and journal stay complete across sessions. Manager doesn't do session work itself; it delegates implementation to spawned workers and defers to them.
---

# Claude manager

Coordinator role for a Claude conversation running inside tmux. Tracks a
registry of sessions, spawns workers on request, and ensures docs and
journal stay complete across sessions — workers contribute as they go,
the manager fills gaps and owns the cross-session view.

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

The grunt of meta-work itself — a read-only search, or drafting an entry
from material the manager hands over — may go to a cheap subagent; the
judgement and any lock-held write stay with the manager. See Delegating
grunt meta-work.

## Delegating grunt meta-work

Meta-work splits into judgement and grunt. Judgement stays with the
manager: deciding what to spawn and record, reconciling state, holding
the cross-session view. Grunt — a read-only content search, or composing
an entry from material the manager hands over — can go to a cheap
subagent, so the manager stays fast and its context stays clean (the
subagent's greps and file reads never enter it). It's the meta-work
counterpart to spawning a worker for substantive work: same instinct,
cheaper mechanism.

Pick the subagent's model with the same rubric as a spawn:

- Read-only or mechanical search (the JSONL hunt in Untracked cold
  resume, registry or journal greps) → `haiku`.
- Composing from a snapshot and notes (the wrap journal draft) →
  `sonnet`. The manager still decides to write it, supplies the
  cross-session context a subagent can't see, and reviews and records
  the result; the subagent only drafts.

Two practical notes: dispatch a fresh subagent with the model set, not a
fork — a fork inherits the manager's model and ignores the override. And
effort isn't a per-dispatch lever for a subagent the way `--effort` is
for a spawned worker, so it follows the subagent's own default; the model
is the lever here.

What never leaves the manager: anything holding the mkdir lock, any
registry write, the tmux lifecycle (spawn, shutdown, wrap kill), and
task-list sync. Delegating lock-held coordination risks a stale-read
clobber and costs more than it saves.

## Task list hygiene

The in-conversation task list mirrors the registry. Sync them in the
same action, never as cleanup later. Conceptual state is encoded as a
prefix on the task description, not in the harness's API status —
this is the formal contract across managers:

- `[active] <session-id>: <ticket or summary>` — live, has a tmux
  container.
- `[paused] <session-id>: ...` — active but parked (`paused` field set,
  tmux still alive).
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
- Pause / unpause → toggle the prefix between `[active]` and `[paused]`.
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

Two lifecycle transitions kill the tmux container, same names on both
sides:

- **Shutdown** — kill the tmux container; keep the registry entry for
  later resumption. Phrasings: "shutdown", "kill that one", "drop
  tmux".
- **Wrap** — final close-out: journal entry, registry removal.
  Phrasings: "wrap up", "complete", "close out", "finish".

**Pause** is different — it flags an active session as parked without
killing anything; tmux and the worker stay alive (see Pause). Phrasings:
"pause", "pause it", "park it"; the inverse is "unpause", "resume it".
For a *paused* (live) session "resume" means unpause; for a *shutdown*
session it means cold resume — the session's state tells them apart.

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
3. **Start the registry watch and attach a `Monitor` to it** (see
   Watching the registry). Spawning the background watch without wiring
   a `Monitor` onto its stdout is inert — the watch logs every worker
   write but the manager never reacts, so self-wraps and shutdowns go
   unnoticed until a manual re-read. If a live watch process for this
   manager already exists (PID file present and PID alive), reuse it
   (re-attach the Monitor); otherwise spawn a fresh one.
4. Install the paused-session switcher binding (see Pause § Switcher
   badge). Idempotent — safe to re-run every invocation.

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
- `model` — model alias the worker was spawned on (`opus`/`sonnet`/
  `haiku`). Recorded intention; informational, since resume restores
  the model itself.
- `effort` — effort level the worker was spawned on
  (`low`/`medium`/`high`/`xhigh`/`max`). Replayed on cold resume;
  unlike the model, effort isn't restored per transcript.
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
- `paused` — timestamp; set on an active entry to mark it parked
  (waiting on review or other external state). Active-only: present
  alongside `tmux_session`, dropped when the session shuts down or
  wraps. Mirrored by a `@cm_paused` tmux option on the session that
  drives the switcher badge (see Pause).
- `wrap_requested` — `true` when a worker has requested wrap; the
  manager fulfils the journal-write phase and removes the entry
- `notes` — string OR a path to a file (typically a journal entry)

Lifecycle state is encoded by which fields are present:

- `tmux_session` set → active.
- `tmux_session` set **and** `paused` set → active, parked.
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
# Now that the lock is held, Read $_reg FRESH (see below), then mutate
# and write with Edit. Don't reuse an earlier in-conversation read.
rmdir "$_lock"
```

**Re-read under the lock.** A manager and its workers share this file,
so the in-conversation Read that an Edit or Write builds on may predate
another process's write. Always Read the registry fresh *after*
acquiring the lock and construct the rewrite from that read — otherwise
a full-file Write clobbers a concurrent write, and an Edit either
matches stale content or fails its `old_string`. The lock without the
re-read still races.

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
model: opus
effort: high
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
`last_touched`, `notes`, ticket/branch updates, `paused`, and the
shutdown/wrap fields (`shutdown`, `resumed_session_id`, `snapshot`,
`resume_state`, `wrap_requested`). Workers may also write their own
snapshot file under `~/.local/state/claude-manager/snapshots/` and
resume_state file under `~/.local/state/claude-manager/resume/`.

**Workers must not touch:** the header block, or any other session's
entry. Workers DO write to journals, wikis, runbooks and other
knowledge stores when their own work warrants it — own-session notes,
wiki updates, runbook additions, anything they can see from inside
the session. The manager's wrap journal entry is additive: it
captures the cross-session/meta view a worker can't (lifecycle,
cross-worker context, what the user reviewed and approved). Wrap is
driven via the `wrap_requested` marker; the manager does the wrap
journal write.

**Manager continues to own:** header refresh for itself; the spawn
flow; the wrap journal entry; ensuring knowledge stores stay complete
(filling gaps the workers couldn't see, adding the cross-session/meta
view); reconciling against `tmux ls`.

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
stdout via `Monitor`. The spawn and the Monitor are a unit — a watch
nobody consumes is inert. Each `changed:` line is a notification — the
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
  removal — see Wrap) as the marker surfaces, not later. Don't let
  markers queue — fulfil each as the watch reports it (or at the next
  idle moment) and keep the pending count at ≤1. When more than one is
  outstanding, surface the count periodically so the backlog stays
  visible. An unfulfilled-wrap backlog is the upstream driver of the
  `wrap_requested`-reopen case (see Reopening from disk).

Fallback: if the watch process is missing on a registry-touching
action (it died, or this is a fresh `claude --resume`), the manager
re-stats the registry on the spot, surfaces any drift, then restarts
the watch. The watch is the fast path; explicit re-stat is the safety
net.

## Spawning a session

1. Clarify ticket, worktree, branch — only what matters.
2. If a worktree is wanted, create it first — worktrees must exist
   before Claude starts inside them (`cwd` cannot change later). Use
   whatever the project rules say. Two cautions:

   - **Never bare `cd` in the manager shell.** The Bash tool's working
     directory persists across calls, so a `cd` into a repo or
     worktree drifts the manager's own cwd and misreports its
     statusline location for the rest of the session. Subshell it
     (`( cd <repo> && wt … )`) or use path flags: `git -C <repo>`,
     `wt -C <repo>`, `tmux … -c <cwd>`.
   - **Base the worktree on a fresh ref when recency matters.** `wt`
     (and bare `git worktree`) branch off the *local* default branch,
     which is often stale relative to `origin`. If the work depends on
     a recent merge, `git -C <repo> fetch origin <default-branch>`
     first, then branch from `origin/<default-branch>` (or
     `reset --hard` the new branch to that ref).
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
   reproductions, prescribed commits, implicit time pressure,
   directive phrasings the worker will fixate on ("read-only, hand
   back after", "ONLY do X"), or assertions about how a system works
   or what the setup/fix mechanism is ("you'll need to run the auth
   flow", "it authenticates via OAuth", "the fix lives in module X").
   Those are manager work disguised as a brief; they bias the worker
   before they've read the room. A stated mechanism is stickier than a
   directive — the worker reads it as established fact, not a claim to
   test — and the manager can't have verified it, since that's worker
   work. It's a guess dressed as context.

   When the task points at a guide, doc, runbook or thread, the brief
   points at it and lets it drive: give the link and the goal, and let
   the worker read the guide for the how. Do not restate, summarize or
   pre-empt what the guide says — least of all how a tool authenticates
   or what the setup steps are. The manager hasn't read it (reading it
   is worker work), so the guide is the authority and any mechanism the
   brief states is just a guess that biases the worker against what the
   guide actually says. If the user's own framing guesses at the
   mechanism, pass it as an assumption to check, not a step to take
   ("user thinks this may need an OAuth step — confirm against the
   guide").

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
6. **Choose the model and effort.** Classify the spawn from surface
   signals only — the meta-only boundary bars investigating to gauge
   complexity, so use what the request already tells you: how the user
   framed it (routine vs hard, any urgency), the kind of work
   (mechanical / implementation / investigation / design / review), its
   breadth and ambiguity, and whether it's the PR-review path.

   Two independent axes: capability need picks the model, task size and
   latency-sensitivity pick the effort. Lean powerful — under-powering
   costs more than over-powering.

   The mapping (retune this as the model set and expectations change;
   the classification above is the stable part):

   - Model: smallest mechanical, mostly just invoking a skill → `haiku`;
     simple, clear-scope work → `sonnet`; real engineering, design,
     cross-system work, debugging, non-trivial review, and the
     when-unsure default → `opus`.
   - Effort (`low`/`medium`/`high`/`xhigh`/`max`): mechanical or small →
     `low`; moderate → `medium`; large, ambiguous or genuinely hard →
     `high`, up to `xhigh`/`max`. Effort varies by size even at a fixed
     model — a small `opus` task runs `--effort low` so it doesn't
     overthink and stay slow.

   Thin signal or a blank spawn defaults to `opus --effort medium`. The
   choice is infrastructure — it does not go into the worker's brief.
7. Start Claude in window 0 pane 0 (the primary worker): launch
   `claude --model <alias> --effort <level>` (the pick from step 6) in
   `${session_id}:0.0`, wait for the TUI input line to be
   ready, send the brief from step 5 (skip the send when the brief is
   blank), then submit. Two traps this flow must handle, whatever the
   mechanics:

   - **Wait on a marker with a timeout.** Poll for a TUI-ready marker
     before sending, and bound the wait so a failed launch surfaces
     instead of hanging. Use tmux interaction patterns or skills for
     the poll; never blind-sleep.
   - **The first Enter often doesn't submit.** A TUI banner — "N setup
     issues", a paste-expand prompt, an MOTD — captures it, so the
     brief lands in the input box but never sends; the spawn looks
     successful from the manager's side while the brief just sits
     there. After submitting, confirm it actually went (busy indicator
     present, input box cleared) and re-send until it does.

   Every marker is TUI-specific — capture the pane first and match what
   the current version renders; don't hardcode a string.
8. Add the session to the registry with `tmux_session: $session_id`,
   plus `model:` and `effort:` from step 6. Add to the visible task
   list (`[active]` prefix).
9. Tell the user how to switch to it (see Switch UX), stating the chosen
   model and effort with a one-line reason. The user can override; ask
   up front only for a genuinely ambiguous or blank spawn.

All session-creating tmux commands include `-d` so spawning sessions
never steals the user's focus. Hard rule.

For **PR review** tasks specifically, the worktree is off the PR's
branch (`gh pr view <N> --json headRefName`). The worker's `review-pr`
skill takes over once the worker starts; the manager just lands it in
the right worktree. Model and effort follow the review class — `opus` at
`high` effort — dropping to `sonnet` or lower effort only for a small or
trivial PR.

## Switch UX

Primary: `prefix+w` picker — interactive list across sessions and
windows. Direct: `tmux switch-client -t <session-id>`. Manager hands
back the session id; the user navigates.

## Reconcile

On demand, diff the registry against `tmux ls`:

- Entry with `tmux_session` set: `tmux has-session -t <tmux_session>`.
  If alive, fine. If not alive, check for an external rename before
  assuming the session died: scan live sessions for one matching this
  entry by registry id, or by a pane's `pane_current_path` equalling
  the entry's worktree/cwd. A match is almost certainly the same
  session renamed outside the manager (`tmux rename-session`) — surface
  it and offer to re-link (update `tmux_session`), don't treat it as
  dead. Otherwise surface and ask: finished, shutdown unexpectedly, or
  unknown? If the live entry carries `paused`, apply the auto-clear
  check (see Pause § Auto-clear).

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

## Detect pane processes

When Shutdown, Wrap, Cold resume or Idle-detection needs to know what's
running in each pane, walk the process tree rather than reading pane
content. The tree gives full argv (so Cold resume can replay any
command verbatim) and avoids the cost and brittleness of capture-pane
sniffing.

Per session, one tmux sweep plus a per-pane process lookup:

```bash
tmux list-panes -s -t "$tmux_session" \
  -F '#{window_index} #{pane_index} #{pane_pid} #{pane_current_path}'
```

For each pane row, identify the foreground process. Usually the pane
is rooted in a shell that has spawned one foreground child (the
running command). Occasionally the shell `exec`'d into a process
directly, in which case `pane_pid` itself is the foreground process.

```bash
# macOS quirks: `comm` returns the full path ("/bin/zsh") and `ucomm`
# returns the basename but pads with trailing spaces to a fixed width
# (which defeats exact case matches). Use `comm` and strip to the
# basename with parameter expansion.
shell_comm=$(ps -p "$pane_pid" -o comm= 2>/dev/null)
shell_comm="${shell_comm##*/}"
case "$shell_comm" in
  bash|zsh|fish|sh|-bash|-zsh|-fish)
    fg_pid=$(pgrep -P "$pane_pid" 2>/dev/null | head -1)
    ;;
  *)
    fg_pid="$pane_pid"
    ;;
esac
[ -n "$fg_pid" ] && ps -p "$fg_pid" -o command= 2>/dev/null
```

Classify each pane by the `ps -p $fg_pid -o command=` output:

- **No `fg_pid`** (idle shell, no foreground child) → resume_state's
  `command:` for this pane is empty.
- **argv matches `node\b.*\bclaude` or `\bclaude(-code)?( |$)`** →
  Claude pane. Record the pid; the JSONL session-id lookup proceeds
  per Shutdown § step 3.
- **Anything else** → capture the full argv verbatim and record it as
  resume_state's `command:` for this pane. Cold resume replays it.
  No hardcoded "known patterns" list — `yarn dev`, `task foo`,
  `npm run watch`, `python -m`, `nvim`, `tail -F`, etc., all captured
  as-is.

Notes:
- `pane_current_command` (from `tmux list-panes`) reports the OS comm
  field, which Claude Code overrides to its version string (e.g.
  `2.1.150`). Fast first signal but not reliable on its own; other
  tools also override their process titles. The `pgrep`/`ps` walk is
  authoritative.
- Cost: one tmux call plus one `ps`+`pgrep` per pane (~50–150ms per
  session of 1–5 panes). Bounded and fast.
- Don't use `tmux capture-pane` for detection. Slow, depends on TUI
  state (vim mode, busy indicator, last frame rendered), false-
  positives against any tool with similar visual conventions.
  Capture-pane is fine for content snapshotting (Shutdown step 2,
  Wrap step 1) and for busy/idle classification of *already
  identified* Claude panes (Idle-detection) — just not for "is this
  Claude".

## Killing a session

`tmux kill-session` on a session a user is attached to detaches them to
the parent shell instead of leaving them in tmux. Before any kill, move
attached clients to another live session (the manager prefers its own):

```bash
# target_session = the session about to be killed.
other=$(tmux list-sessions -F '#{session_name}' \
  | grep -vx "$target_session" | head -1)
if [ -n "$other" ]; then
  tmux list-clients -t "$target_session" -F '#{client_name}' \
    | while read -r c; do tmux switch-client -c "$c" -t "$other"; done
fi
tmux kill-session -t "$target_session"
```

If it's the only session on the server, the kill drops to the shell no
matter what — nothing tmux can do. The manager runs this against an
entry's `tmux_session`; a self-wrapping/shutting-down worker runs it
against its own `$src_session` (it is killing the session it sits in,
so the user is almost always the attached client).

## Shutdown

Shutdown = kill the tmux session; keep the registry entry so the
session can be cold-resumed later via the manager. Sits between
active and Wrap (final, journal entry written, entry removed).
Flexible wording: "shutdown", "kill that one", "drop tmux".

**Mechanics:**

1. **Discover structure.** Window layouts:

   ```bash
   tmux list-windows -t "$tmux_session" \
     -F '#{window_index} #{window_name} #{window_layout}'
   ```

   Per-pane foreground process and command: see § Detect pane
   processes. The output identifies which panes are Claude (each
   with a discovered pid) and the verbatim command for any other
   pane that has one.

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

3. **Find Claude session IDs for every Claude pane** identified in
   step 1. For each:

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
   Idle-shell panes (per § Detect pane processes) get `command:`
   empty — auto-replaying an idle shell on resume is noise. The primary
   Claude pane's `command:` carries `--effort <effort>` from the registry
   `effort` field, but not `--model` — resume restores the model itself
   (see Cold resume). Example:

   ```markdown
   # Resume state: eng-1234

   shutdown: 2026-05-22

   ## window 0: claude
   layout: 5fe4,200x50,0,0,0

   ### pane 0
   cwd: ~/code/.../eng-1234
   command: claude --effort high --resume abc-123
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
   - Removes `tmux_session` and `paused` (if set).

6. **Kill the tmux session** (after the lock is released), moving any
   attached client off it first — see § Killing a session.

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

4. For each window, run (n-1) splits, where n = the number of panes
   recorded for that window in the resume_state file:

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
   verbatim (the primary worker's recorded command also carries
   `--effort`; see Shutdown). Other panes get their recorded command.

   **Model vs effort on resume.** Model is not re-passed: resume restores
   the session's last model itself, so re-passing `--model` would override
   a mid-session `/model` change. Effort has no such per-transcript
   restore, so the recorded `--effort` is replayed deliberately — it
   restores the spawn-time choice at the cost of overriding a mid-session
   `/effort` change. That trade is chosen because reverting to the default
   effort is worse than losing an occasional mid-session tweak, and the
   live effort can't be recovered at shutdown (the transcript stores no
   readable effort level). Two limits: the wrap_requested-reopen and
   untracked-cold-resume paths below have no recorded effort, so they come
   back at the default; and `max`, being session-only, survives a resume
   only where this recorded `--effort max` replays it.
7. Update the registry under lock:
   - Add `tmux_session: $session_id`.
   - Remove `shutdown`, `snapshot`, `resume_state`,
     `resumed_session_id`.
   - Update `last_touched`.

   Delete the on-disk snapshot and resume_state files — the live
   session supersedes them.
8. Update the visible task list: prefix `[shutdown]` → `[active]`.

**Partial-failure recovery.** The registry rewrite (step 7) is the last
step, so any failure before it leaves the entry in `shutdown` already —
correct, nothing to undo there. What's left behind is a half-built tmux
session. If `new-window` / `split-window` / `select-layout` / `send-keys`
fails (worktree missing, malformed layout), kill the partial session and
leave the registry in `shutdown` so the user can retry from a clean
slate:

```bash
tmux kill-session -t "$session_id" 2>/dev/null
```

Surface the failing command's error; don't half-update the registry.

**Manual escape hatch.** A user can always run `claude --resume <id>`
from the worktree to bring the primary worker back without the
surrounding scaffold. That route doesn't update the registry; the
next reconcile catches the drift.

## Reopening from disk

Cold resume (above) rebuilds a `shutdown` entry from its `resume_state`
file. Two other cases reach a session that exists on disk but has no
`resume_state` to rebuild from. Both are manager-only.

### Untracked cold resume

The session isn't in the registry at all — its entry was wrapped and
removed, or never recorded — but the user wants the conversation back.
Find the Claude JSONL by content, then resume it.

macOS gotcha throughout: encoded project dirs start with `-`, which
breaks bare `stat` / `basename` / grep flag parsing. Always use
absolute paths and `--` separators.

1. Encode the worktree/cwd and grep its project dir for the ticket id
   or topic (encoding per Shutdown § step 3):

   ```bash
   encoded=$(echo "$cwd" | sed 's|[/._]|-|g')
   proj_dir="$HOME/.claude/projects/$encoded"
   grep -rl -- "<ticket-id-or-topic>" "$proj_dir"/*.jsonl 2>/dev/null
   ```

2. Rank candidates by `mtime` + hit-count + first-prompt match.
   Exclude the manager's own JSONL and any live manager's JSONL — a
   manager conversation matches topic greps for the sessions it
   spawned. Resolve each live manager's project dir from its
   `manager:` header pane (pane → `pane_current_path` → encoded dir).

3. Recreate the tmux session and resume the chosen id (`-d` so focus
   isn't stolen):

   ```bash
   tmux new-session -d -s "$session_id" -n "$session_id" -c "$cwd"
   tmux send-keys -t "${session_id}:0.0" "claude --resume <id>" Enter
   ```

4. Register the entry with `tmux_session`, `started`, `last_touched`;
   add the task list entry with `[active]` prefix.

### Reopen a wrap_requested entry

The entry carries `wrap_requested: true` but shouldn't have wrapped —
wrap was premature (e.g. more review work landed). The higher-context
case: the registry entry already holds `resumed_session_id` and
`snapshot`, so there's no JSONL hunt.

1. Recreate the tmux session and resume the primary worker:

   ```bash
   tmux new-session -d -s "$session_id" -n "$session_id" -c "$cwd"
   tmux send-keys -t "${session_id}:0.0" \
     "claude --resume <resumed_session_id>" Enter
   ```

2. Rewrite the entry under the lock: clear `wrap_requested`, re-add
   `tmux_session: $session_id`, update `last_touched`.
3. Set the task list prefix back to `[active]`.

The manager owes no journal entry until the session next wraps.
Keeping the wrap backlog at ≤1 (see Watching the registry) is what
stops premature wraps from accumulating into this case.

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
   writing. Otherwise proceed. The drafting can go to a `sonnet`
   subagent (see Delegating grunt meta-work); the manager supplies the
   snapshot, notes and cross-session context, then reviews and records.
4. Mark the visible task list entry `completed`, then delete it from
   the list.
5. Remove the entry from the registry.
6. Kill the tmux session if it's still alive, moving any attached
   client off it first — see § Killing a session.

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
and removes the entry) is intentional: the worker tears its own session
down promptly on wrap rather than leaving it alive until a manager
happens to process the marker.

**Forked workers.** Wrap records only the calling (primary) worker's
`resumed_session_id`, by design: wrap is final, so per-fork resume ids
serve nothing — a wrapped session isn't resumed. The forks' context
isn't lost; the snapshot walks every window and pane (step 2), so the
manager sees each fork's output when writing the journal. Contrast
Shutdown, which records every Claude pane's `claude_session_id`
precisely because the session will be rebuilt.

## Pause

Pause flags an active session as parked — waiting on review or other
external state — without killing anything. The tmux session and worker
stay alive; only the label changes, so a parked session is easy to skip
in the `prefix+w` switcher and in the task list. It is a modifier on the
active state, distinct from Shutdown (kills tmux) and Wrap (final).

Both sides drive it. A worker self-serves via `/claude-manager-pause`
(a toggle); the manager pauses or unpauses any session on the user's
request. Either path lands the state in the same two places: the
registry `paused` field and a per-session `@cm_paused` tmux option.

The registry rewrite uses the same lock and re-read-under-lock
discipline as everything else (see Registry). A worker acting on its own
entry resolves and matches it the same way the shutdown/wrap flow does
(`claude-manager-end/FLOW.md` § Common preamble).

**Pause** (active → parked):

1. Under the lock, add `paused: <today>` to the entry and update
   `last_touched`. A reason, if given, goes in `notes`.
2. Set the marker: `tmux set-option -t <tmux_session> @cm_paused 1`.
3. Ensure the switcher binding is installed (§ Switcher badge).
4. Set the task-list prefix to `[paused]`.

**Unpause** (parked → active):

1. Under the lock, remove `paused` from the entry; update
   `last_touched`.
2. Clear the marker: `tmux set-option -u -t <tmux_session> @cm_paused`.
3. Set the task-list prefix back to `[active]`.

`@cm_paused` lives on the tmux session, so it vanishes when the session
is killed — shutdown and wrap need no marker cleanup, only the `paused`
field drop.

### Auto-clear

During a reconcile or idle-detection pass, if a parked session's primary
Claude pane reads **busy** (per § Idle-detection query — `esc to
interrupt`, spinner), the manager unpauses it: the session is clearly
active again. Idle-at-prompt is *not* activity — a parked session sits
idle, so don't auto-unpause on idle. Explicit unpause is the reliable
path; auto-clear is best-effort.

### Switcher badge

The marker renders in `prefix+w` via a custom `choose-tree` format,
installed at runtime rather than added to the user's `tmux.conf` — so
the feature travels with the skill:

```bash
tmux bind-key w choose-tree -Zw -F '#{?session_format,#{?#{@cm_paused},⏸ paused · ,}#{session_windows}w#{?session_attached, (attached),},#{?window_format,#{window_name}#{window_flags},#{pane_current_command}}}'
```

Install it idempotently on manager invocation and whenever a pause is
set, so the badge shows whenever a parked session exists, with or
without a manager running. It is purely additive — identical to the
stock `choose-tree -Zw` except for the `⏸ paused` badge on sessions
whose `@cm_paused` is set. The session name is untouched (it stays equal
to the registry id); `choose-tree` draws the name and shortcut key
itself, and the format only adds the trailing description.

Caveats, accepted as the price of keeping it skill-owned: the rebind is
global and reverts to the user's `tmux.conf` on a tmux **server
restart**, re-installing on the next invocation or pause — so between a
restart and the next manager activity, badges don't render. If the user
has customised their own `w` binding, the runtime install overrides it.

## Knowledge work

The manager doesn't own the project's recordkeeping outright; workers
contribute as they work (wiki pages, runbook additions, own-session
notes), and the manager ensures it stays complete across sessions.

When the project rulebook (already in Claude's context on startup)
points to a journal, wiki, investigations dir or similar, the
manager writes into them at:

- Session wrap (the wrap journal entry — manager-side, capturing the
  cross-session/meta view a worker can't see).
- Mid-session decisions, conventions or lessons worth outliving the
  worker that a worker has surfaced but not yet recorded.
- Filling gaps when a worker's contribution is incomplete or scoped
  too narrowly.
- On request.

Workers write directly when their own work warrants it; the manager
doesn't gate that. Read the relevant store's schema before writing.

## Idle-detection query

"Which workers are waiting for input?" — for each registry session
with `tmux_session` set, identify Claude panes per § Detect pane
processes, then capture each Claude pane's recent tail (use tmux
interaction patterns or skills for the capture):

```bash
tmux capture-pane -p -J -t "<tmux_session>:<w>.<p>" -S -30
```

Heuristics (Claude Code TUI), applied only to known-Claude panes:

- **Idle**: trailing `> ` prompt; no `esc to interrupt`; no spinner.
- **Busy**: `esc to interrupt` present; spinner; streaming output.

Heuristic, not authoritative. Report best-effort with evidence.

Other queries (switch back, what's around) are direct registry
reads/writes — update `last_touched` and the visible task list as
needed.

## Resource awareness

If workers report claimed ports, dev servers etc, capture under the
session's notes. Warn on plausible conflicts when spawning a new
session. No automatic scanning.

## Ending the manager session

Distinct from the per-session Shutdown and Wrap above: this closes out
the *coordinator* conversation itself. Without it, registry entries
keep pointing at tmux sessions that won't survive the user ending the
day or restarting the machine, and the watch process is left orphaned.

Trigger phrasings: "wrap up the manager", "I'm done for the day",
"shut the manager down".

1. **Walk every live entry** (those with `tmux_session`). For each,
   confirm it's actually alive (`tmux has-session`) and resolve it
   with the user — one of:
   - **Leave running** — the user keeps the tmux session up past the
     manager. Nothing to change; the entry stays active and the next
     manager invocation re-adopts it on its registry read. If the
     machine restarts before then, that invocation's reconcile catches
     the now-dead entry.
   - **Shut down** — kill tmux, keep the entry for cold resume (per
     Shutdown).
   - **Wrap** — final close-out (per Wrap).
2. **Reconcile dead entries** the walk surfaces (entry with
   `tmux_session` set but no live session) — per Reconcile.
3. **Stop the watch.** Kill the watch process and remove its PID file
   so a future manager doesn't read it as live:

   ```bash
   sanitised=$(echo "$mgr_pane" | tr ':.' '--')
   pidfile="$HOME/.local/state/claude-manager/watch.${sanitised}.pid"
   [ -f "$pidfile" ] && kill "$(cat "$pidfile")" 2>/dev/null
   rm -f "$pidfile"
   ```

4. **Drop this manager's header line.** Under the lock, remove the
   `manager: $mgr_pane` line for this pane from the header block (leave
   other managers' lines alone), then release.
5. Exit. Live worker sessions left running are fine — they're
   self-contained tmux sessions; only the coordinator is ending.

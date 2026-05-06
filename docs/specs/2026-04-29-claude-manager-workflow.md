# Claude manager workflow

Spec for a "manager" Claude conversation that coordinates parallel work
across tmux. Tracks a registry of sessions, spawns them on request, and
keeps adjacent journal, wiki and log artefacts up to date as work moves
through them.

This is one piece of the harness, not the whole of it. It must compose
with the rest (AGENTS.md rules, other skills, normal Claude conversations
running without it).

## Goals

- Reduce mental load when juggling several sessions of work, especially
  when work pauses for hours or days.
- Let me say "let's also work on X" and have Claude set up the
  environment (worktree, branch, tmux window or tmux session, Claude
  inside, extra panes) without me running the steps.
- Keep a durable picture of what I'm working on, what's parked, and the
  trail of context, so I can return days later without rebuilding state
  from memory.
- Let me close down my environment without losing track of in-flight
  work.

## Non-goals

- Hiding tmux state from `tmux ls`. All tmux sessions and tmux windows
  stay visible and attachable normally.
- Replacing my multiplexer or providing a TUI of its own. Manager is a
  Claude conversation, nothing more, and is assumed to run inside a
  normal tmux session.
- Replacing existing workflows. Worktrees, branches, journal and wiki
  remain handled by their own tools; manager orchestrates and records.

## Manager doesn't execute session work

The manager only does meta-work. Anything substantive — investigation,
analysis, code reading, code editing, debugging, config changes, build
or test runs, tracing system state — is worker work, even when it's
small, even when it's "just" readonly. Doing any of it from the manager
conversation pollutes its context and erodes its purpose. The default
response to a substantive request is "let me spawn a worker for that",
not "let me take a quick look".

It *does* own all the meta-work that workers typically don't pick up on
their own: harness knowledge, journal logging, wiki upkeep, session and
ticket tracking. The manager owns the journal (and other knowledge
stores) end-to-end; the registry is a shared channel — see the
Registry section.

In scope:

- The header on the registry; full registry reads/writes for
  manager-initiated operations.
- tmux operations on session containers (new-window, move-window,
  list-windows, capture-pane for state observation).
- Journal entries, wiki updates, harness notes, per the project's
  schemas. This includes journal entries triggered by a worker's
  `wrap_requested` marker.
- Conversation-level planning, clarification and routing: deciding
  what worker to spawn, what context it needs, where its output goes.
- Light meta updates I explicitly request (a one-line ticket/todo
  edit, a registry-adjacent setting), provided it's unambiguously
  meta.

Out of scope, deferred to a worker by default:

- Investigating a bug, a PR, an approach, or a question about how
  something works. Even readonly. Even "just to discuss".
- Reading code to form an opinion.
- Editing code, configs or settings beyond a trivial one-liner I've
  explicitly asked for.
- Running tests, builds, linters, or anything that produces
  substantive output the manager would have to interpret.
- Any task where the manager would learn something it didn't already
  know. That learning belongs in the worker's context, not the
  manager's.

When in doubt, spawn a worker. The cost of a fresh worker is low; the
cost of a polluted manager is high.

The manager may read worker pane output (`tmux capture-pane`) to
understand session state, but does not send keys or prompts to workers
unless I explicitly ask it to. The registry takes the place of any
direct prompting between worker and manager — workers communicate
state changes by editing their own entries, the manager observes via
its registry watch.

## Terminology

I'll often say "task" or "session" for an item in the registry; treat
them as the same. "tmux session/window/pane" means the literal tmux
object; bare "session" might mean either, infer from context.
**Manager** and **worker** are roles for Claude conversations. The
**registry** is the markdown file at
`~/.local/state/claude-manager/sessions.md`.

A registry session has one tmux container at a time — either a tmux
window inside the manager's tmux session, or its own tmux session — and
one or more workers (Claude conversations in panes) inside that
container.

## Registry

Single markdown file. `# Sessions` heading; an optional block of
header `key: value` lines (one per active manager); then one
`## <session-id>` per session with its own `key: value` block and
optional prose.

Example:

```markdown
# Sessions

manager: hbz:1.0

## eng-1234-payment-bug
ticket: ENG-1234
tmux_window_id: @42
tmux_session: payment-bug
claude_panes: 0
worktree: ~/code/repo-foo/_wt/eng-1234
branch: fix/eng-1234-payment-bug
started: 2026-04-29 14:00
last_touched: 2026-04-29 16:20
notes: ~/code/journal/2026-04-29-eng-1234.md

## explore-cmux
tmux_window_id: @17
claude_panes: 0
started: 2026-04-29 11:00
last_touched: 2026-04-29 11:45

Looking at cmux as a tmux alternative. No worktree, just poking from
the hbz repo. Not committed to anything.

```

Recognised header fields:

- `manager` — `<tmux-session>:<window>.<pane>` for an active manager.
  One line per manager. Each manager refreshes its line on invocation
  and on registry-touching actions. Stale lines are tolerated;
  presence is best-effort, not a guarantee the pane is alive.

Recognised session fields:

- `ticket` — free-form ID or URL
- `tmux_window_id` — stable tmux window id (`@N`), assigned at
  spawn. Survives renumbering and renaming, so this is the canonical
  way to find the window's current location:
  `tmux list-windows -a -F '#{window_id} #{session_name}:#{window_index}'`.
- `tmux_session` — present only when the window lives in a standalone
  tmux session (parked). Human-readable hint that survives a glance
  read of the registry without running tmux. Derivable from
  `tmux_window_id` lookup.
- `claude_panes` — comma-separated list of tmux pane indices in the
  session's tmux window where workers (Claude) run. Default `0`.
- `worktree` — absolute path
- `branch` — branch name
- `started`, `last_touched`, `shutdown` — timestamps, format flexible
- `resumed_session_id` — Claude `--resume` token captured at shutdown
  or wrap
- `snapshot` — path to pane snapshot captured at shutdown or wrap
- `resume_target` — expected resume date (free-form)
- `wrap_requested` — `true` when a worker has requested wrap; the
  manager fulfils the journal-write phase and removes the entry
- `notes` — string OR a path. When a path, the manager treats that
  file as the canonical narrative for the session.

Optionality matters: sessions can exist before any of these are
decided.

The registry has no explicit status field. Lifecycle state is
encoded by which fields are present:

- `tmux_window_id` set, no `tmux_session` → active.
- `tmux_window_id` set, `tmux_session` set → parked.
- No `tmux_window_id`, `shutdown` + `resumed_session_id` set →
  shutdown.
- No `tmux_window_id`, `wrap_requested: true` → wrap in progress
  (worker has marked, manager hasn't fulfilled).

Wrapped sessions are removed entirely. Recency is `last_touched`;
current state is whatever tmux says when the id is looked up.

Reads/writes are full-file. The registry is shared between the
manager and its workers — both sides may write. Mutual exclusion uses
a `mkdir` lock on `${registry}.lock` (cross-platform; `flock` is
Linux-only). The format is forgiving: an unknown field or stray
prose under a session is preserved on rewrite, not lost.

### Workers writing to the registry

Workers may write to their own session entry only. Allowed fields:
`last_touched`, `notes`, ticket/branch updates, the tmux-location
swap a self-park performs, and the shutdown/wrap fields (`shutdown`,
`resumed_session_id`, `snapshot`, `wrap_requested`). Workers may also
write to their own snapshot file under
`~/.local/state/claude-manager/snapshots/`.

Workers must not touch the header, any other session's entry, or the
journal/wiki. Wrap is driven via the `wrap_requested` marker; the
manager fulfils the journal write.

### Manager registry watch

The manager runs a background watch process (`stat`-poll loop on the
registry's mtime, consumed via the `Monitor` tool) so worker writes
surface in the manager's task list live, not just on the manager's
next turn. PID file is keyed by manager pane address so multiple
managers don't trample each other. If the watch dies, the manager
detects on its next registry-touching action via explicit re-stat,
surfaces drift, and restarts the watch. Watch state is per-manager
and per-conversation; nothing on the file system survives the
manager's shutdown beyond the registry itself.

### `wrap_requested` two-phase flow

Wrap is the only lifecycle transition that touches the journal, and
the journal schema lives on the manager side. Workers initiate wrap
by writing `wrap_requested: true` along with snapshot path,
`resumed_session_id`, and any context for the journal as `notes`,
then killing the tmux container. The manager's watch fires; the
manager reads the project's journal schema, writes the entry, and
removes the registry entry.

If the manager isn't running when the worker wraps, the marker
persists. The next manager invocation processes it during reconcile.

## Visible task list

The registry file is canonical. The manager additionally mirrors live
sessions into Claude's in-conversation task list (TaskCreate /
TaskUpdate in Claude Code) so I can see them at a glance in the UI.

Conceptual state lives in the task description as a formal prefix —
this is the contract that matters across managers, not the harness's
own status set:

- `[active] <session-id>: ...` — has a tmux container.
- `[parked] <session-id>: ...` — `tmux_window_id` set, plus
  `tmux_session` for human readability.
- `[shutdown] <session-id>: ...` — `shutdown` field set, no tmux
  fields.
- `[wrap requested] <session-id>: ...` — transient; worker has
  marked, manager hasn't fulfilled.

The harness status is plumbing: every registry entry maps to
`in_progress`; the manager flips to `completed` only at wrap
fulfilment, just before removing the task. Don't extend the harness
status set to match the prefixes — they're orthogonal.

The list is conversation-scoped, so a fresh manager conversation
rebuilds it from the registry on invocation. It must be kept in sync
with the registry on every state change; out-of-sync UI is worse than
no UI.

## Manager responsibilities

### Spawning a session

1. Clarify what's known: ticket? worktree wanted? branch name?
2. If a worktree is wanted, create it first. Worktrees must exist
   before Claude starts inside them: `cwd` cannot change later, and
   restarting Claude is expensive in context. Manager is agnostic
   about *how* the worktree is created (project rules decide), but
   the timing is hard.
3. By default, spawn the session as a new tmux window in the manager's
   own tmux session, named after the session id (`-n <session-id>`),
   with `cwd` set to the worktree (or repo root if none). Capture the
   stable window id at spawn (`tmux new-window -d -P -F '#{window_id}'`).
   The manager itself occupies window 1 of its tmux session; spawned
   tasks go after. Gappy indices are fine — entries identify windows
   by `tmux_window_id`, not by index, so a wrap or shutdown in the
   middle leaves a hole that never needs filling.
4. Start Claude in pane 0 of that tmux window (the primary worker),
   plus any extra panes the project's CLAUDE.md/AGENTS.md describes.
   If any of those extra panes also runs Claude, append its index to
   `claude_panes`.
5. Add the session to the registry, recording `tmux_window_id` and
   `claude_panes`.

All `tmux new-window` and `tmux move-window` invocations include `-d`
so spawning, parking or reopening sessions never steals my focus.
Hard rule.

For **PR review** tasks specifically, the worktree is off the PR's
branch (resolved via `gh pr view <N> --json headRefName`). The worker
runs the `review-pr` skill once started; the manager's job is just to
land it in the right worktree. The layered walkthrough, stance and
variants live in that skill, not here.

### Lifecycle: park, shutdown, wrap

Three transitions, same vocabulary on both sides; flexible wording
mapped to the canonical mode before acting.

**Park** = move the session's tmux container out of the manager's
window list into a standalone tmux session. Reversible. Park can be
initiated by me asking the manager directly (manager runs the
mechanics) or by the worker self-serving via `/claude-manager-park`
(worker runs the move and updates the registry; manager observes via
the watch). End state is identical either way.

```bash
tmux new-session -d -s <name> -n placeholder
tmux move-window -d -s <manager-tmux-session>:<idx> -t <name>:0 -k
```

Reverse direction (merge a standalone tmux session back as a window):

```bash
tmux move-window -d -s <name>:0 -t <manager-tmux-session>: -k
tmux kill-session -t <name>
```

Both stay visible to `tmux ls`. Nothing is hidden in the mitsuhiko
sense.

**Shutdown** = kill the tmux container; keep the registry entry. The
worker captures its panes, resolves its own Claude conversation's
JSONL session id, writes `shutdown`, `resumed_session_id`, and
`snapshot` to its registry entry, then kills the tmux container.
The manager-initiated path runs the same mechanics from the manager
side. Either way, resume later via `claude --resume <id>` from the
worktree (or `cwd`).

**Wrap** = final close-out, two-phase. Worker captures snapshot,
gathers any context worth carrying into the journal, writes
`wrap_requested: true` along with `snapshot` and `resumed_session_id`
to its registry entry, then kills the tmux container. Manager's
watch picks up the marker, writes the journal entry per the
project's schema, and removes the registry entry. If I ask the
manager to wrap a session directly, the manager runs both phases
itself — the worker need not be alive.

Park, shutdown and wrap don't change with how they were initiated:
the registry state is the source of truth, the watch keeps the
manager's view current.

### Attaching and switching

Manager hands me the right command, or runs `switch-client` when
appropriate. It does not multiplex on my behalf. If a session lives
as a tmux window in the manager's tmux session, switching is just
`select-window`. If it lives as its own tmux session, attaching is
via the manager itself (`switch-client`) or whatever client I prefer.

### Reconciling with reality

The watch auto-triggers a reconcile on any worker write, so manual
reconciliation is mostly only needed for tmux-side drift the watch
can't see.

On demand, manager diffs the registry against `tmux ls` and the
manager's own window list:

- Entries in the registry but missing from tmux: the tmux container
  has been closed.
  - If `shutdown` is set, leave the entry alone — it was already
    shutdown.
  - If `wrap_requested: true` is set and the watch missed it,
    trigger the manager-side wrap-fulfilment now (journal write,
    entry removal).
  - Otherwise, ask me: finished or shutdown? Finished triggers
    Wrap. Shutdown triggers the Shutdown flow on the manager side.
- tmux sessions or windows present in tmux but not in the registry:
  prompt me to import or ignore. Manager does not silently take
  ownership of anything it didn't create.

### Importing an external tmux session

When I have a tmux session I started outside the manager and want to
bring it under management, the manager:

1. Confirms the tmux session exists.
2. Asks for a session id and any other useful context (ticket, branch,
   worktree, which panes run Claude).
3. Captures the window id
   (`tmux display-message -p -t <name>:0 '#{window_id}'`) and adds
   the session to the registry with `tmux_window_id`,
   `tmux_session: <name>`, and `claude_panes`.
4. Mirrors into the visible task list.
5. Hands me the appropriate `switch-client` or `attach` command, or
   re-imports as a tmux window in the manager's tmux session if I'd
   rather have it inline.

### Answering queries

The manager should answer common queries directly without bothering a
worker. Notable ones:

- **"Which workers are waiting for input?"** Capture each entry in
  each session's `claude_panes` (`tmux capture-pane -p -t
  <tmux-target>.<pane> -S -30`) and apply heuristics: idle Claude
  shows a trailing `> ` prompt with no `esc to interrupt` and no
  spinner; busy Claude shows `esc to interrupt` and/or running tool
  output. Report best-effort with evidence; this is a heuristic, not
  authoritative.
- **"Switch back to X"** — find the session, update `last_touched`,
  hand me the right attach/switch command for its current tmux
  location.
- **"What am I working on?" / "What's around?"** — surface the live
  registry sessions; flag any divergence between registry and tmux
  state.

### Resource awareness

Manager keeps a lightweight register of resources workers have claimed
(ports, dev servers, ad-hoc db handles) when workers tell me about
them and I relay it. On spawning a new session, manager warns about
likely conflicts. Expectations are modest: there is no automatic
process scanning, and the only mitigation tool is "remind workers to
share where possible".

This is opt-in noise reduction, not enforcement.

### Wrap mechanics

Whether triggered by me asking the manager directly or by a worker's
`wrap_requested` marker, the manager runs:

1. Capture a final snapshot for each pane in `claude_panes` (or every
   pane, when killing the container) via `tmux capture-pane -p -t
   <target>.<pane> -S -200`. If the worker has already written a
   snapshot path, use that instead.
2. Write the journal entry per the project's schema, using the
   snapshot and any `notes` from the registry entry. If notes are
   thin and there's no obvious narrative from snapshot + recent git
   activity, ask me a focused question before writing.
3. Set the visible task list entry to `completed`.
4. Remove the entry from the registry. The journal is the durable
   record from here.
5. Kill the tmux container if it's still alive: look up
   `tmux_window_id` against `tmux list-windows -a` to find its
   current location, then `tmux kill-window`. If the worker already
   killed it, this step is a no-op. No renumber afterwards — gappy
   indices are fine. Renumber on demand only (see below).

### Renumber

On demand only. Gappy indices in the manager's tmux session don't
matter for the registry — entries identify by `tmux_window_id`. If
I want indices tidy, I ask the manager and it runs
`tmux move-window -r -s <manager-tmux-session>`. No registry
follow-up.

### Knowledge work

The manager owns the project's recordkeeping, not just transition
events. That covers journal entries, wiki updates, harness notes
(rules to lift into AGENTS.md, lessons learned), and per-incident
scratchpads where the project supports them. The events that warrant
a write include session wrap, but also significant context changes
mid-session, decisions that should outlive the worker, and standing-up
of new conventions.

Discovery flow:

1. From the manager's working directory, read the project's
   CLAUDE.md/AGENTS.md.
2. Identify any documented knowledge stores it points to (journal,
   wiki, investigations, ADRs, etc.). Each store typically has its
   own schema rulebook in its own directory.
3. Read the schema before writing. Follow it.

The skill itself encodes none of this. `~/code/` happens to expose a
journal/wiki/investigations triplet, but other projects will look
different, and the skill must not assume any particular shape.

## Multiple managers

Multiple managers in different directories are allowed. Each one:

- Reads the same registry on startup.
- Notices other managers' sessions and treats them as out of scope but
  visible.
- Can hand off sessions by editing the registry, or by mutual
  agreement during a conversation routed through me.

Not the default pattern, but the design must not make it harder than
it needs to be.

## Project integration

The manager skill is generic. Project-specific behaviour (journal
location, wiki structure, worktree conventions, default panes per
session type) lives in the project's CLAUDE.md/AGENTS.md, read when
manager is invoked from within that project. Keeps the skill portable
and avoids leaking work-specific paths into a public dotfiles repo.

## What we deliberately don't build

- **No CLI tool.** The model can read the registry, run `tmux ls`,
  edit files, and run tmux commands directly. A CLI is only worth
  adding if a concrete need shows up (heavy contention, inspection
  from non-Claude shells).
- **No standalone daemon.** The registry watch is a per-manager
  background process tied to the manager's conversation lifetime; it
  is not a system service, not a cross-conversation singleton, and
  has no responsibilities beyond emitting `changed:` events when the
  registry's mtime advances.
- **No hidden sockets.** All tmux state remains visible to `tmux ls`
  and attachable from any client.
- **No automatic tmux discovery hook.** A tmux session-create hook
  that pings manager is possible later; for now, the registry watch
  notices worker writes but doesn't see external tmux changes —
  those still go through manual reconcile.
- **No automatic window renumber after wrap or shutdown.** Gappy
  indices in the manager's tmux session are fine — entries identify
  windows by stable `tmux_window_id`, so a hole left by a wrapped
  session never needs filling. Renumber is on demand only, when the
  user asks for it (see Renumber).
- **No separate tmux skill, for now.** The existing `# tmux` rules in
  `agents/AGENTS.md` are shared by manager and workers and don't
  need more. If they grow, lift to a skill.
- **No tmux-prompt channel between workers and the manager.** The
  registry is the only structured channel. Workers communicate by
  editing their own entries; the manager observes via the watch.
- **No status fields.** No `active`/`paused`, no `done`. Live
  sessions are in the registry; wrapping removes them. Lifecycle
  state is encoded by which fields are present (`shutdown`,
  `wrap_requested`).

## Related work

This spec supersedes `docs/plans/2026-04-23-isolated-tmux-experiment.md`.
The private-socket model from `mitsuhiko/agent-stuff` (a separate tmux
server under `$TMPDIR/claude-tmux-sockets/`, invisible to normal
`tmux ls`) is rejected: hiding tmux sessions from `tmux ls` conflicts
with this design's hard requirement that everything stays visible.
Helper-script patterns (`wait-for-text`-style polling, structured tmux
helpers) may still be borrowed inside the manager skill, not vendored
as a skill.

## Open questions

- **What "aware of external sessions" looks like in practice.** The
  registry watch closes the worker-write side; tmux-side drift still
  needs manual reconcile. Possible later: a tmux hook touching a
  marker file the manager checks.
- **Journal/wiki contract.** The skill needs a small, well-defined
  contract that project rules implement. Pin down when first used in
  `~/code/`.

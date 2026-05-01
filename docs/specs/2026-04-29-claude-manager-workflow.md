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
ticket tracking, recordkeeping across the registry. Workers rarely show
interest in this work, so making it the manager's primary responsibility
ensures it actually happens.

In scope:

- Reading and writing the session registry.
- tmux operations on session containers (new-window, move-window,
  list-windows, capture-pane for state observation).
- Journal entries, wiki updates, harness notes, per the project's
  schemas.
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
unless I explicitly ask it to.

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
tmux_session: payment-bug
claude_panes: 0
worktree: ~/dev/mv/repo-foo/_wt/eng-1234
branch: fix/eng-1234-payment-bug
started: 2026-04-29 14:00
last_touched: 2026-04-29 16:20
notes: ~/dev/mv/journal/2026-04-29-eng-1234.md

## explore-cmux
tmux_window: hbz:3
claude_panes: 0
started: 2026-04-29 11:00
last_touched: 2026-04-29 11:45

Looking at cmux as a tmux alternative. No worktree, just poking from
the hbz repo. Not committed to anything.

```

Recognised header fields:

- `manager` — `<tmux-session>:<window>.<pane>` for an active manager.
  One line per manager. Each manager refreshes its line on invocation
  and on registry-touching actions. Workers (e.g. `claude-manager-park`)
  read these lines to locate a manager. Stale lines are tolerated;
  workers verify by capture-pane and fall back to scanning if a line
  no longer points at a Claude TUI.

Recognised session fields:

- `ticket` — free-form ID or URL
- `tmux_session` — tmux session name when the session lives in its own
  tmux session
- `tmux_window` — `<tmux-session>:<index>` when the session lives as a
  tmux window inside another tmux session, typically the manager's
- `claude_panes` — comma-separated list of tmux pane indices in the
  session's tmux window where workers (Claude) run. Default `0`. Used
  by idle-detection queries.
- `worktree` — absolute path
- `branch` — branch name
- `started`, `last_touched` — timestamps, format flexible
- `notes` — string OR a path. When a path, the manager treats that
  file as the canonical narrative for the session.

Optionality matters: sessions can exist before any of these are
decided.

The registry has no status fields. Live sessions are in the file;
wrapping a session removes it from the file (the journal carries the
record from then on). Recency is `last_touched`; whether a session is
currently in front of the user is loosely correlated with tmux
location but not reliable enough to encode.

Manager always reads the file fresh, applies its edits, writes once.
Multiple managers coordinate via `flock` on the file. The format is
forgiving: an unknown field or stray prose under a session is preserved
on rewrite, not lost.

## Visible task list

The registry file is canonical. The manager additionally mirrors live
sessions into Claude's in-conversation task list
(`TaskCreate`/`TaskUpdate`) so I can see them at a glance in the UI.

Status mapping is simple: every registry entry is `in_progress`. When
a session is wrapped, the manager sets its task list entry to
`completed` (so I see the check) and then removes it from the
registry.

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
   with `cwd` set to the worktree (or repo root if none). The manager
   itself occupies window 1 (or whatever `base-index` makes the first
   window); spawned tasks go after.
4. Start Claude in pane 0 of that tmux window (the primary worker),
   plus any extra panes the project's CLAUDE.md/AGENTS.md describes.
   If any of those extra panes also runs Claude, append its index to
   `claude_panes`.
5. Add the session to the registry, recording `tmux_window` and
   `claude_panes`.

All `tmux new-window` and `tmux move-window` invocations include `-d`
so spawning, parking or reopening sessions never steals my focus.
Hard rule.

For **PR review** tasks specifically, the worktree is off the PR's
branch (resolved via `gh pr view <N> --json headRefName`). The worker
runs the `review-pr` skill once started; the manager's job is just to
land it in the right worktree. The layered walkthrough, stance and
variants live in that skill, not here.

If I want the session to live as its own tmux session (typically to
attach to it from a separate client, or just to get it out of the
manager's window list), the manager moves it on request:

- Create a detached target tmux session with a placeholder window.
- `move-window` the session's tmux window into the target tmux
  session.
- Drop the placeholder.
- Update the registry: drop `tmux_window`, add `tmux_session: <name>`.

The reverse (move it back as a tmux window in the manager's tmux
session) is symmetric via `move-window`. Both directions stay on the
same tmux server, fully visible to `tmux ls`. Nothing is hidden in the
mitsuhiko sense. These moves don't change session status: usually
moving a session out of the manager's window list means I've parked it,
but not always, so the registry doesn't try to encode that.

### Attaching and switching

Manager hands me the right command, or runs `switch-client` when
appropriate. It does not multiplex on my behalf. If a session lives
as a tmux window in the manager's tmux session, switching is just
`select-window`. If it lives as its own tmux session, attaching is
via the manager itself (`switch-client`) or whatever client I prefer.

### Reconciling with reality

On demand, manager diffs the registry against `tmux ls` and the
manager's own window list:

- Entries in the registry but missing from tmux: the tmux container
  has been closed. Ask me whether the session is finished or just
  parked. Finished triggers Wrap. Parked leaves the entry alone — a
  dormant entry is fine.
- tmux sessions or windows present in tmux but not in the registry:
  prompt me to import or ignore. Manager does not silently take
  ownership of anything it didn't create.

### Importing an external tmux session

When I have a tmux session I started outside the manager and want to
bring it under management, the manager:

1. Confirms the tmux session exists.
2. Asks for a session id and any other useful context (ticket, branch,
   worktree, which panes run Claude).
3. Adds the session to the registry with `tmux_session: <name>` and
   `claude_panes: <n[,m,...]>`.
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

### Wrap-up

Wrap is the one explicit lifecycle transition. On wrap:

1. For each pane in `claude_panes`, capture
   `tmux capture-pane -p -t <target>.<pane> -S -200` for a final
   snapshot.
2. Write the journal entry per the project's schema, including the
   snapshot and any prose notes carried in the registry entry.
3. Set the visible task list entry to `completed`.
4. Remove the entry from the registry. The journal is the durable
   record from here.
5. Close the tmux container. For a `tmux_window` in the manager's
   tmux session: kill the window, then `tmux move-window -r -s
   <manager-tmux-session>` to renumber so windows stay sequential.
   Refresh any other registry entries in that tmux session by mapping
   their session-id (used as window name) back to the current
   `window_index` via `tmux list-windows`. For a `tmux_session`:
   `tmux kill-session -t <name>`; no renumber needed.

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

The skill itself encodes none of this. `~/dev/mv/` happens to expose a
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
- **No daemon.** Manager is interactive; reconciliation is on demand.
- **No hidden sockets.** All tmux state remains visible to `tmux ls`
  and attachable from any client.
- **No automatic discovery hook.** A tmux session-create hook that
  pings manager is possible later; v1 reconciles only when asked.
- **No separate tmux skill, for now.** The existing `# tmux` rules in
  `agents/AGENTS.md` are shared by manager and workers and don't
  need more. If they grow, lift to a skill.
- **No remote prompting of workers.** Manager reads worker panes; it
  doesn't send keys to them unprompted.
- **No status fields.** No `active`/`paused`, no `done`. Live
  sessions are in the registry; wrapping removes them. Recency is
  `last_touched`; current state is whatever tmux says.

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

- **Lock semantics on the registry.** `flock` is the obvious answer;
  confirm availability everywhere this runs and pick a fallback if
  not.
- **What "aware of external sessions" looks like in practice.** v1:
  nothing automatic, manager scans on demand. Possible later: a tmux
  hook touching a marker file the manager checks.
- **Journal/wiki contract.** The skill needs a small, well-defined
  contract that project rules implement. Pin down when first used in
  `~/dev/mv/`.
- **Repo organisation.** Where the manager skill lives in the
  dotfiles repo, and how the install path lines up with personal
  skill discovery. To be decided when implementation starts.

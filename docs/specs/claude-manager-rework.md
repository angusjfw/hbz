# claude-manager rework: pi support and a leaner skill

Status: draft for review. Nothing implemented.

Rework of `claude/skills/claude-manager*` so the same skill family manages
pi coding agent sessions as well as Claude Code sessions, and so the skill
is short and mechanical enough for weaker models to run reliably.

## Requirements

- The manager supports pi, not only Claude Code.
- Registry sessions stay tmux sessions, not windows, and everything stays
  on one tmux server.
- pi and Claude Code are separate setups: a pi session only ever involves
  pi.
- In tmux terms a worker is a pane; a window is a collection of panes.
- Whether workers message each other isn't a rule the manager enforces.
- The skill is too long for weaker models (pi often runs smaller or local
  models).

Constraints:

- One tmux session per registry session, all on the one tmux server.
- A managed session is one harness or the other, never mixed. A pi manager
  spawns pi workers; a Claude manager spawns Claude workers.
- A worker is a pane. A window is a collection of panes.

Decisions taken in review of the brief:

- **One registry file**, with a `harness:` field per entry.
- **Keep the `claude-manager-*` names** and the state dir. The neutral
  rename (`amux` in `claude-manager-followups.md`) is a separate step.
- **pi registry watch via a small pi extension.**
- **No fixed model table.** Short guidelines in the skill; the manager
  reads the harness's available models once per invocation and asks when
  unsure.
- **The manager loads only when invoked explicitly**
  (`disable-model-invocation: true`, which both harnesses support). The
  worker lifecycle skills stay model-invocable, so "wrap up" in a worker
  still works.
- **`cm` is on PATH.** Source lives in the skill directory
  (`claude/skills/claude-manager/scripts/cm`); `make ai` symlinks it to
  `~/.local/bin/cm`, as `make session-leds` does for `agent-status`. Clients
  outside the family (pane-team) find it by discovery rather than by a path
  into the claude-manager skill.
- **`cm-watch` goes in `pi/extensions/`.** `make pi` already symlinks
  everything there.
- **One worker skill.** pause, shutdown, wrap and END-FLOW.md merge into a
  single `claude-manager-worker` skill that takes a mode.
- **Coordination stays outside the family.** Coordination is a pattern
  skill (`pi/skills/pane-team` for pi; a loosened rewrite of
  `coordinator-worker` for Claude later) that uses `cm` as a client.
- **The pane-team hookup is in scope** (Phase 3), so pane-team workers in
  a managed session are registered and come back on resume. See
  Coordination clients.
- **Model floor.** The manager runs on the stronger enabled models. The
  worker skill must work on any model, including small local ones, and
  stays at about 80 lines.

## Non-goals

- Renaming the family, the state dir, `@cm_paused` or the PID files.
- Changing `coordinator-worker`, or anything under `pi/` beyond the Phase 3
  hand-off points (`cm-watch`, pi settings' `skills` list, and pane-team's
  spawn and finish steps).
- Worker-to-worker messaging rules. The manager doesn't forbid it; the
  manager itself still only sends to workers when asked.
- Mixed-harness sessions, or a manager managing the other harness.
- Switcher demotion of paused sessions (separate follow-up).

## Verified facts (pi 0.87.1, Linux, 2026-09-26)

Checked against pi's bundled docs and in a scratch tmux session on the
user's server. Unverified items are marked.

| Topic | Claude Code | pi |
|---|---|---|
| Mint id at spawn | `--session-id <uuid>` | `--session-id <id>`: opens that id in the current project, **creating it if absent** (so a wrong cwd or session dir silently gives a blank session). Letters, digits, `.`, `_`, `-`. |
| Resume | `claude --resume <id>` | `pi --session-id <id>` from the **same cwd** (lookup is per project). `--session <id>` also works but offers a fork prompt for a cross-project match. |
| Initial prompt | positional `claude "<text>"` (documented; verify with `--session-id` in Phase 1) | positional `pi "<text>"`: submits on start (verified) |
| Model on resume | restored | restored (verified) |
| Effort on resume | not restored; replay `--effort` | thinking level restored (verified); don't replay |
| Effort flag | `--effort low…max` | `--thinking off\|minimal\|low\|medium\|high\|xhigh\|max`, clamped to the model |
| Display name | `-n/--name` | `-n/--name`; sets pane title `π - <name> - <cwd basename>` (verified) and is stored as a `session_info` entry. With `--session-control`, the name is also a **global** alias: `control.ts` `createAliasSymlink` replaces any existing `~/.pi/session-control/<name>.alias`. Whether `--session-id` alone restores the name on resume is unverified (Phase 2 check). |
| Session control | n/a | `--session-control` (mitsuhiko/agent-stuff `control.ts`) enables the per-session socket pane-team uses. Currently on only via `alias pi="pi --session-control"` in `zsh/.zshrc`, which doesn't apply to lines tmux types into a non-interactive context or to scripts. |
| Process argv | full argv visible in `ps` | **overwritten to `pi`**: `/proc/<pid>/cmdline` and `ps` show only `pi`, and tmux-resurrect saves only `pi` (verified) |
| `pane_current_command` | version string, e.g. `2.1.150` | `pi` |
| Children of the pane shell | `claude` on the pane tty + tool shell on another tty | same shape: `pi` on the pane tty + `zsh` on another tty (verified) |
| Transcript dir | `~/.claude/projects/<cwd with [/._] → ->/` | `<session dir>/` + `--` + cwd without the leading `/`, with `/`, `\` and `:` replaced by `-`, + `--` (docs; `/` → `-` verified). Lossy (a `-` in the path is ambiguous), so matches are confirmed against the header `cwd`. Session dir: `--session-dir`, then `PI_CODING_AGENT_SESSION_DIR`, then the `sessionDir` setting, then `~/.pi/agent/sessions`. |
| Transcript file | `<id>.jsonl` | `<timestamp>_<id>.jsonl`; header line carries `id` and `cwd` |
| agent-status store | hooks | `pi/extensions/agent-status.ts` bridge; store keyed by the real pi session id with `pane_id` (verified) |
| `$TMUX_PANE` in tool shell | set (fails under remote-control) | set (verified via `!echo`) |
| Busy marker | spinner + `(… · ↓ n tokens)` | `⠧ Working` spinner in the input box's top border; retries show `⠇ Retrying (n/3) … (escape to cancel)` |
| Input-box ghost text | yes (dim placeholder) | none seen |
| Background task + Monitor | yes | none built in. Extensions can `fs.watch` and `pi.sendMessage(…, {triggerTurn, deliverAs: "followUp"})` (see `examples/extensions/file-trigger.ts`) |
| In-conversation task list | TaskCreate etc. | none |
| Subagents with model choice | yes | none built in; a subagent extension is in progress in `pi/` (parallel work) |
| Skill command | `/claude-manager-worker wrap` | `/skill:claude-manager-worker wrap` (arguments are appended as a user request); `disable-model-invocation: true` supported |
| Skill discovery | `~/.claude/skills/` | `skills` list in pi settings (already points into `~/.claude/skills/…`), `~/.agents/skills/` |

Corrections to `docs/plans/claude-manager-harness-agnostic-notes.md`:
pi *does* have a mint-id flag (`--session-id`), and pi restores thinking
level on resume.

Other findings that affect the rework:

- `uuidgen` is not installed on this Linux machine, so the current spawn
  step fails here. Use `/proc/sys/kernel/random/uuid` or `python3 -c
  'import uuid; print(uuid.uuid4())'`.
- Because pi's argv is overwritten, argv can't identify a pi pane's session
  in either live detection or crash recovery. See Identity below.

## Design

### Terms

- **Registry session**: one entry in the registry, one tmux session, one
  harness.
- **Worker**: a pane running an agent. The **primary** worker is the first
  window's pane 0.
- **Window**: a collection of panes. Nothing is keyed by window.

The skill's "tmux window in the manager's session" container option is
removed from Terminology; it's stale since the sessions pivot.

### Skill family layout

The family is one thing with one shared layer and one entry point per
role. A skill directory holds one `SKILL.md`, so each role that needs its
own name and invocation rule gets its own directory.

Shared layer, in `claude/skills/claude-manager/`:

- `scripts/cm`: all mechanics (below), installed on PATH as `cm`.
- `scripts/test_cm.py`: stdlib `unittest` cases (see Tests).
- `REFERENCE.md`: registry format, fields, lifecycle states, resume_state
  format. The one place these are described.

Entry points:

- `claude-manager` (`SKILL.md` in the same dir): the manager role.
  Explicit-only.
- `claude-manager-worker`: the worker role, model-invocable. Modes
  `pause` (toggle), `shutdown`, `wrap`.

Entry points call `cm` on PATH and read `REFERENCE.md` by path
(`~/.claude/skills/claude-manager/REFERENCE.md`), never sections of each
other's text. Coordination skills are clients of `cm` only, and aren't
part of the family.

### Registry changes

- New field `harness: claude|pi`. An entry without it is `claude`, so
  existing entries need no migration.
- Header `manager:` lines gain the harness: `manager: <addr> harness=pi`.
- Every other field keeps its name and meaning. `resumed_session_id` and
  `worker:` lines hold the harness's own session id.
- resume_state pane blocks write `session_id:`; readers accept the old
  `claude_session_id:`.
- A manager acts only on entries of its own harness. It reads all entries
  so reconcile doesn't offer to import the other harness's tmux sessions,
  and it lists the other harness's sessions only when asked.

### `cm`: one script for the mechanics

The mechanics that are currently shell snippets inside the prose move into
one CLI at `claude/skills/claude-manager/scripts/cm` (python3, stdlib
only; python3 is already a dependency via `read-input-box.py`), symlinked
to `~/.local/bin/cm` by `make ai`. Each subcommand does one thing, prints plain lines or
JSON, and exits non-zero with a message on failure.

| Subcommand | Replaces |
|---|---|
| `cm whoami` | `$TMUX_PANE` fast path + ancestry-walk fallback; prints pane id, tmux address, session, and the matching registry entry id (or none) |
| `cm reg show [<id>]` | ad hoc registry reads |
| `cm reg new <id> k=v…` / `set <id> k=v…` / `unset <id> k…` / `rm <id>` | lock + fresh re-read + Edit + release. `@header` addresses the header block; `k+=v` appends a repeatable line and `unset k=v` removes one matching line, so `cm reg set @header manager+="<addr> harness=pi"` replaces the old manager add/drop. |
| `cm reg worker add\|drop <id> <session-id> [cwd=…] [label=…]` | hand-written `worker:` lines |
| `cm panes <tmux-session>` | Detect pane processes: per pane `window pane pane_id cwd kind session_id id_source command`, kind ∈ `claude\|pi\|cmd\|idle` |
| `cm snapshot <tmux-session> <out>` | the three copies of the snapshot loop; input-box classification on Claude panes only |
| `cm resume-state <tmux-session> <out>` | building resume_state by hand |
| `cm spawn --harness <h> --id <id> --cwd <dir> …` | tmux session creation, id minting, launch line, registry entry |
| `cm spawn --into <tmux-session> --label <name> --cwd <dir> [--window <name>\|--split <pane>] [--brief-file <f>] …` | new agent pane inside an existing managed session: mints the id, launches, adds the `worker:` line (with `label=`) to the owning entry, prints the pane id and the final name. The harness comes from the entry. The client interface for coordination skills (see Coordination clients). |
| `cm rebuild <id>` | Cold resume steps 2–6, with the kill-on-partial-failure rollback. For pi, first checks every pane's transcript exists (see Harness adapter) and aborts before creating anything if one doesn't. |
| `cm kill <tmux-session>` | Killing a session (move clients via `agent-deck switch`, fall back to `switch-client`) |
| `cm transcripts <harness> <cwd> [--grep <text>] [--name <n>]` | the two JSONL hunts; lists candidates with mtime and hit count. For pi, keeps only files whose header `cwd` equals `<cwd>`, and `--name` matches `session_info` names. |
| `cm reconcile` | read-only drift report: dead entries, renamed sessions, unregistered agent panes, missing `worker:` lines |
| `cm watch` | the stat-poll loop (Claude's background watch) |

Registry writes go only through `cm reg`. Each call takes the mkdir lock,
re-reads the file, applies the change, writes it back and releases the lock
in one process. Timestamps come from the clock (`k=now`). Each write also
records the writer's `$TMUX_PANE` in `~/.local/state/claude-manager/.last-writer`
(used by the pi watch to skip its own writes). The lock no
longer spans tool calls, so the "re-read under the lock" and "release
explicitly" instructions disappear from the skill. Unknown fields, header
lines and prose are kept. The on-disk format is unchanged, so the other
harness, an older skill version and a human can all still edit it.

Judgement stays in the model: what to spawn, the brief, journal entries,
what to do about drift. `cm reconcile` reports and `cm rebuild` executes;
neither decides.

### Harness adapter

Inside `cm`, one small table per harness covers the differences listed in
Verified facts: launch line, resume line, effort flag, whether to replay
effort, transcript dir rule, pane detection, busy marker. The skill text
names a harness only where its behaviour differs (watch, task list,
subagents, skill command form).

Launch lines built by `cm spawn`:

- Claude: `claude --session-id <uuid> --model <m> --effort <e> "$(cat <brief>)"`
- pi: `pi --session-control --session-id <uuid> [--model <m>] --thinking <t> --name <name> "$(cat <brief>)"`,
  where `<name>` is the registry id for a primary worker and the prefixed
  label (Coordination clients) for a `--into` worker.

`--session-control` is explicit in every pi line built by `cm`, not left
to the shell alias.

Resume lines built by `cm rebuild`:

- Claude: `claude --effort <e> --resume <id>` (as now)
- pi: `pi --session-control --session-id <id> [--name <name>]`, typed in
  the pane whose cwd is the recorded one. `--name` is passed if the Phase 2
  check finds the name isn't restored on its own.

Because `pi --session-id` creates a missing session instead of failing,
`cm rebuild` checks each pi pane before building anything: resolve the
session dir (`--session-dir` if configured, else
`PI_CODING_AGENT_SESSION_DIR`, else the `sessionDir` setting, else
`~/.pi/agent/sessions`), find `<sessiondir>/<slug>/*_<id>.jsonl`, and
confirm its header `cwd` equals the recorded cwd. If any check fails, it
aborts with the missing id and the path it searched, before creating any
tmux state.

### Spawn

The brief goes on the command line from a file, instead of typing into
the TUI once it's ready. `cm spawn` writes the brief to
`~/.local/state/claude-manager/briefs/<id>.md`, creates the tmux session
(`-d`), and types the launch line into the primary pane's shell. The
readiness polling and "the first Enter often doesn't submit" handling go
away. After launching, `cm spawn` waits (bounded) for the agent-status
store to show the new session id on that pane, or for `cm panes` to see
the agent. On timeout it reports and leaves the pane for inspection.

Everything else in Spawning stays as it is: worktree first, never a bare
`cd`, name collision check, the brief provenance test, and the switch
handle.

### Identity of agent panes

A pane's session id comes from the first of these that has it:

1. The registry (`resumed_session_id`, `worker:` lines). This is primary
   for both harnesses because ids are minted at spawn.
2. The agent-status store, `claudes[<id>].pane_id == pane_id`. Works for
   both harnesses while the pane is live.
3. argv `--session-id` / `--resume` (Claude only).
4. `cm transcripts` content hunt (the last resort).

For pi, route 3 doesn't exist, so the `worker:` line discipline carries
more weight. `cm reconcile` flags any agent pane that has no id from
routes 1–2.

pi crash recovery (a CRASH-RECOVERY.md paragraph): the resurrect save
has each pane's title, which `--name` set to `π - <name> - <dir>`. Map
`<name>` to a session id through the entry's `worker:` labels (or the
registry id for the primary), then through `session_info` names in that
cwd's transcripts (`cm transcripts pi <cwd> --name <name>`), then the
copied agent-status store.

### Watch

- Claude: unchanged in behaviour. `cm watch` run in the background with a
  `Monitor` on its output.
- pi: a pi extension `cm-watch` registers a tool the manager calls
  (`cm_watch start|stop`). It does nothing unless started, so ordinary pi
  sessions are unaffected. When started it `fs.watch`es the registry's
  directory (not the file, since a rewrite may replace it) and debounces.
  On a change it reads `.last-writer` and ignores the change if the writer
  is its own pane. Otherwise it sends a custom message with
  `{triggerTurn: true, deliverAs: "followUp"}`, which pi already queues
  when a turn is running, so there's no resend logic. It stops on
  `session_shutdown`. The manager's reaction is the same as for Claude:
  re-read, diff, act (wrap fulfilment included).
- Fallback for both: re-check the registry on any registry-touching action
  when the watch isn't running (as now).

The PID-file rules apply only to Claude's watch. pi's watch lives and
dies with the pi process.

### Task list

Claude keeps the in-conversation task list contract. pi has no task list.
There the registry is the only view, and "what's running" is answered
with `cm reg show` / `cm reconcile`. The skill states the task-list rules
once, conditional on the harness having a task list, rather than on every
step.

### Model and effort

- **Claude:** keep the current rubric (haiku / sonnet / opus × effort), as
  short guidelines.
- **pi:** one sentence in the skill: on invocation, read `defaultModel`,
  `defaultThinkingLevel` and `enabledModels` from pi settings once. The skill gives
  guidelines only: default model for most work; a stronger enabled model
  for design, debugging and review; a small or local model for
  mechanical, skill-invoking work; thinking level by task size. If the
  mapping isn't obvious from the model names, it asks the user. Spawn
  reports the choice with a one-line reason, as now.
- Recorded as `model:` / `effort:` on the entry. For pi, `effort` holds the
  thinking level and isn't replayed on resume.

### Grunt delegation

"Delegating grunt meta-work" becomes conditional: if the harness offers
subagents with a model choice, delegate as now; otherwise do it inline.
It doesn't name a pi mechanism.

### Worker skill

- `claude-manager-worker` replaces `claude-manager-pause`,
  `-shutdown`, `-wrap` and END-FLOW.md. One `SKILL.md` with a short
  common preamble (`cm whoami`, find own entry, idempotency guard) and a
  section per mode. Description is harness-neutral ("agent session") and
  lists the trigger phrasings for each mode. About 80 lines, written to
  run on any model: every step is a `cm` call or one plain decision.
- Steps call `cm whoami`, `cm snapshot`, `cm panes`, `cm resume-state`,
  `cm reg …`, `cm kill`. The lock pattern, JSONL hunt and duplicated
  snapshot loop are gone.
- The resume-id step no longer assumes "the newest JSONL in my project dir
  is me". It uses the registry, then the agent-status store, then
  `cm transcripts`.
- Invocation: `/claude-manager-worker wrap` in Claude,
  `/skill:claude-manager-worker wrap` in pi, or plain words ("wrap up",
  "park it", "shut this down"). The old command names go away.
- The manager's own Shutdown and Wrap run the same `cm` calls. Where the
  two sides differ (the manager's journal write for wrap), each skill
  says so in its own text.

### Coordination clients

Coordination skills (pane-team now, a Claude coordinator later) aren't
part of the family. Inside a managed session they spawn and finish workers
through `cm`, so the registry knows every agent pane; outside one they
keep their own mechanics. For pane-team (Phase 3 hand-off points):

- **Spawn.** If `cm whoami` finds a registry entry for the current tmux
  session, spawn with `cm spawn --into <session> --label <name> --cwd …
  --brief-file …` and use the printed pane id and name. Otherwise keep the
  raw `tmux split-window` it has now.
- **Names.** pi aliases are global, so `cm spawn --into` always names a pi
  worker `<registry-id>-<label>` and rejects the spawn if
  `~/.pi/session-control/<that name>.alias` already exists. pane-team
  addresses workers by the name `cm` prints.
- **Finish.** Before `tmux kill-pane`, run `cm reg worker drop <entry>
  <session-id>`. A worker pane left open keeps its line.
- **Resume.** `cm rebuild` relaunches every pi pane with
  `--session-control` (and the name, per the Phase 2 check), so pane-team
  can talk to its workers again after a cold resume or crash recovery.

### Manager's own tmux session

The auto-numbered rename becomes `<harness>-manager` (`claude-manager`
or `pi-manager`), with the same guards. The `agent-status clear` of the
old name stays.

### Skill shape

Targets, so the skill fits weaker models:

- `SKILL.md` ≤ ~400 lines (currently 1537). Each flow is a numbered list
  of `cm` calls and decisions, with the expected output and the condition
  to stop on.
- Rationale moves out of the steps. Where a reason prevents a known
  mistake, one sentence stays next to the step. Longer history lives in
  the specs already in `docs/specs/`.
- On-demand companions: `REFERENCE.md` (shared layer), `CRASH-RECOVERY.md`
  (rare) and a new `HARNESSES.md` holding the per-harness notes a model
  needs to read (skill command form, watch, task list, model guidelines).
  The mechanical differences stay in `cm`.
- The hard boundary, brief provenance test and input-box warning stay
  close to verbatim. They are judgement, not mechanics, and the most
  important text in the skill.

## Tests

`scripts/test_cm.py`, stdlib `unittest`, run with
`python3 -m unittest` from `scripts/`. Cases:

- Registry round-trip: parse and rewrite leaves unknown fields, header
  lines, stray prose and field order untouched; `set`, `unset`, `+=`,
  `worker add|drop`, `@header`.
- Lock: a held lock blocks a second writer; timeout reports the holder;
  the lock is released on error.
- Legacy keys: an entry without `harness` reads as `claude`; resume_state
  with `claude_session_id:` parses.
- resume_state parsing against a copy of the live registry and resume
  files (copied into a temp dir; never the live ones).
- pi slug and header-cwd filtering in `cm transcripts`.

## Migration

- Existing entries need no rewrite (`harness` defaults to `claude`).
- On first invocation of the new manager: drop `manager:` header lines
  whose pane no longer exists, and delete `watch.*.pid` files whose PID is
  dead. `cm reconcile` reports both; the manager removes them.
- Remove the old worker skill dirs and their `~/.claude/skills`
  symlinks (Phase 1).

## Phases

Each phase is committed separately and leaves the skill working.

1. **`cm` + lean skill, Claude only, no behaviour change.** Build `cm`
   (all subcommands, Claude adapter). Write REFERENCE.md, rewrite
   SKILL.md around `cm`, and replace the three worker skills and
   END-FLOW.md with `claude-manager-worker` (removing the old skill dirs
   and their `~/.claude/skills` symlinks). Add the `make ai` symlink to
   `~/.local/bin/cm` and the tests. Replace `uuidgen`. Verify on a scratch session: spawn →
   pause → unpause → shutdown → cold resume → wrap, plus `cm reconcile`
   against a hand-made drift. Set `disable-model-invocation: true` on the
   manager. One behaviour change is included: the brief
   moves onto the command line, if the check that `--session-id` plus a
   positional prompt submits passes. Otherwise TUI send stays for Claude.
2. **pi adapter.** Add `harness:`, the pi adapter (explicit
   `--session-control`, the transcript check in `cm rebuild`), the
   identity routes, HARNESSES.md and the CRASH-RECOVERY.md pi paragraph.
   Check whether `pi --session-id <id>` restores the session's name on
   resume, and settle the resume line accordingly.
   Verify the same lifecycle with a pi worker spawned from a Claude shell
   using `cm` directly (no pi manager yet).
3. **pi manager and pane-team hookup.** Builds on `pi/skills/pane-team`
   and `pi/extensions/subagent`. Add the `cm-watch` extension in
   `pi/extensions/`, add the two skills to pi settings' `skills` list, and
   change pane-team's spawn and finish steps per Coordination clients.
   Verify a pi manager end to end: a worker self-wrap caught by the watch,
   and a pane-team worker that is registered, survives shutdown and cold
   resume, and can be messaged again afterwards.

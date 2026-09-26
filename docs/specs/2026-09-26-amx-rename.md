# amx: rename, auto-registration, manager-less wrap, board

Status: agreed scope, building.

The manager family serves Claude Code and pi, so `claude-manager` names the
wrong thing and `cm` says nothing. This renames it all to `amx` (agent
multiplexer), the name `2026-06-12-amx.md` chose for the same CLI, and
builds the rest of that spec, which was never built:

- a guard against killing tmux sessions that aren't the entry's own;
- session hooks that log every agent session and register tmux ones
  automatically;
- wrap done by the worker, without a manager;
- `amx-board`, an on-demand cross-session review.

## Names

| Now | After |
|---|---|
| CLI `cm` (inside the manager skill) | `amx`, source at `amx/amx` (a tool gets a top-level dir), linked to `~/.local/bin/amx` |
| `scripts/test_cm.py` | `amx/test_amx.py` |
| manager skill's `REFERENCE.md` | `amx/REFERENCE.md`, beside the CLI every skill calls |
| skill `claude-manager` | `amx` (explicit-only: `/amx`, `/skill:amx`) |
| skill `claude-manager-worker` | `amx-worker` |
| (new) | `amx-board` |
| state `~/.local/state/claude-manager/` | `~/.local/state/amx/` |
| env `CM_STATE_DIR` | `AMX_STATE_DIR` |
| tmux option `@cm_paused` | `@amx_paused` |
| pi `cm-watch.ts`, tool `cm_watch` | `amx.ts` (watch tool `amx_watch` + session hooks) |
| manager tmux session `<harness>-manager` | `amx-<harness>` |

Unchanged: `pane-team` (a client of `amx`), the registry format and field
names, agent-status `park`/`clear`. `HARNESSES.md` and `CRASH-RECOVERY.md`
stay with the `amx` skill.

## Shared sessions and the kill guard

A tmux session is **shared** if its name is a bare number (tmux
auto-named it) or a registry `manager:` line points into it. Shared
sessions are where the user works; amx never owns them.

- `amx shutdown` and `amx wrap` kill the tmux session only if its name
  equals the entry id and it isn't shared. Otherwise they finish the
  registry side, leave tmux alone, and say why.
- `amx kill` refuses a shared session.
- The session hooks never register a shared session (below).

## Session hooks

`amx hook --harness <h>` reads one event as JSON on stdin (Claude Code's
hook format: `hook_event_name`, `session_id`, `cwd`, `source` or
`reason`). Claude Code runs it from SessionStart and SessionEnd hooks; the
pi extension sends the same shape on `session_start` and on
`session_shutdown` with reason `quit`. It always exits 0 and never
prompts; errors go to `hook-errors.log`.

**Log.** Every event appends a line to `session-log.jsonl`: time, event,
harness, session id, cwd, tmux session and pane (if any), git branch,
source/reason. Also written by `amx wrap` and `amx reg rm` (the removed
entry), so the log keeps a record of every entry that existed.

**Register** (start events, inside tmux, not shared):

- No entry for this tmux session, and no entry with its name: create
  one with `auto: true`, `harness`, `tmux_session`, `cwd`, and the id as
  `resumed_session_id` (primary pane) or a `worker:` line.
- Entry exists and is active: record the id if it isn't there. The
  primary pane's id replaces `resumed_session_id` (a Claude `/clear`
  starts a new id in the same pane); any other pane gets a `worker:`
  line.
- An entry with the name but a different or no `tmux_session`: log only;
  reconcile reports it.

**Unregister** (end events, entry active, same pane): an `auto` entry
whose primary ended is removed; a non-primary pane's `worker:` line is
dropped. Anything else is left alone. End events are best effort: an
agent killed with its tmux session usually sends none, which is why
shutdown and wrap don't rely on them.

**Track.** `amx track <id> [--as <name>]` makes an auto entry a tracked
one (drops `auto`), optionally renaming the entry and its tmux session
together. It replaces importing.

`amx ls [-a]` lists entries with state, harness, ticket and busy state;
auto entries only with `-a`. `amx log [--orphans]` lists sessions from
the log; `--orphans` shows ones that ended with no entry and no wrap,
for a retro-wrap.

## Wrap without a manager

The worker does the whole wrap:

1. `amx reg show <id>` for the resume pointers.
2. Write the journal entry per the project's schema, if it has one,
   carrying the resume pointers (ids and cwds) and the snapshot path
   `snapshots/<id>.txt`.
3. `amx wrap <id>`: snapshot, remove the entry (logged), release the LED
   key, kill tmux (guarded).

No `wrap_requested` marker. A manager, if running, sees the entry go and
mentions it. Old entries that still carry `wrap_requested` show up in
reconcile, and a manager or the board finishes them.

## amx-board

A model-invocable skill for "what's open", "review my sessions", "tidy
up". Runs in any session and does judgment on records only:

- `amx ls -a`, `amx reconcile`, `amx log --orphans`.
- Proposes, and does on agreement: tracking auto entries worth keeping,
  wrapping or retro-wrapping dead and orphaned sessions (journal from
  the log, snapshot and transcript), sweeping shutdown entries past
  their `resume_target`, filling journal gaps.

The standing manager (`/amx`) stays for coordinator days: it spawns,
watches and resumes. The board is the same judgment as a one-off query.

## Migration

`amx migrate`, idempotent:

1. Nothing to do if `~/.local/state/amx/` exists or the old dir doesn't.
2. Under the old lock, move the directory and rewrite registry path
   fields (`snapshot`, `resume_state`, `notes` when it's a path) to the
   new prefix.
3. Drop `manager:` lines and `watch.*.pid` files. No manager survives
   the rename.
4. On live paused sessions set `@amx_paused` and unset `@cm_paused`.

`make claude` and `make pi` run it after linking and remove
`~/.local/bin/cm`, so `make install` on the Mac switches everything,
live entries included. The Claude hooks go into `settings.json.example`
(merged by `make claude`); pi gets them through the extension. Run it
with no manager or worker mid-transition.

## Phases

1. **CLI.** `amx/` with the renamed CLI, tests and REFERENCE.md; kill
   guard; `hook`, `ls`, `log`, `track`; `wrap` without the marker and
   with logging; `migrate`. Tests cover the guard, hook registration and
   unregistration, track, log and migrate against a copy of the live
   state.
2. **Skills and wiring.** `amx`, `amx-worker`, `amx-board`; pane-team's
   calls; the pi extension; Claude hook wiring; Makefile; comments in
   `tmux/.tmux.conf`, agent-status and the session-LED README; AGENTS.md.
3. **Live.** `make ai` here, which migrates. Verify: an unmanaged Claude
   and pi in a new tmux session auto-register and unregister on quit;
   nothing registers in session `0`; `amx track --as`; shutdown of the
   existing entry still rebuilds; a worker wraps itself end to end; the
   board runs.

## Out of scope

- The `tasks.md` agenda that `2026-06-12-org-mode-assessment.md` placed
  in the board.
- `amx send` / `amx output`, switcher demotion.
- Old specs keep their names and paths; `claude-manager-followups.md`
  loses its rename item when this lands.

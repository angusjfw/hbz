# Claude manager: paused session state

Design for the `paused` lifecycle facet on claude-manager sessions.
Resolves the "Paused state for sessions" follow-up in
`claude-manager-followups.md`. Companion to
`2026-04-29-claude-manager-workflow.md` and
`2026-05-22-claude-manager-sessions-pivot.md`.

## Goal

A session waiting on review or other external state should be easy to
*ignore* — distinguishable at a glance in the `prefix+w` switcher and to
the manager — without killing tmux or losing worker state. The marker is
primarily for the user scanning the switcher; the manager reads it too.

## State

`paused` is a modifier on the active state, not a replacement. A paused
session still has a live tmux session and a running worker; only its
label changes. It is distinct from shutdown (kills tmux, keeps the
registry entry for cold resume) and wrap (final, journal + entry
removed). It is also distinct from the retired "park" concept, which
moved sessions out of the manager's window list.

Both sides can pause and unpause:

- **Worker** self-serves via `/claude-manager-pause` from its own pane.
- **Manager** pauses/unpauses any session on the user's say-so.

## Registry encoding

Reuses the existing "state = which fields are present" convention in
`sessions.md`. One new optional session field:

- `paused: <date>` — timestamp. Presence means paused; the timestamp is
  the "parked since" marker and the baseline for auto-clear. Removed on
  unpause.

Lifecycle encoding gains one line:

- `tmux_session` set **and** `paused` set → active, paused.

`paused` is added to the recognised session fields and to the
worker-writable field list (workers self-pause). A reason, if any, goes
in the existing `notes` field — no dedicated field. `paused` is an
active-only field: the shutdown and wrap rewrites drop it along with
`tmux_session`.

## tmux marker — no rename

The session name stays equal to the registry id (the primary key is
untouched). The marker is carried by a per-session tmux user option and
rendered by a custom `choose-tree` format.

- Pause sets `tmux set-option -t <session> @cm_paused 1`; unpause clears
  it with `tmux set-option -u -t <session> @cm_paused`. The option dies
  with the session, so no cleanup is needed on shutdown/wrap.
- `prefix+w` renders a `⏸ paused` badge on any session whose
  `@cm_paused` is set, via a `-F` format on `choose-tree`.

Verified end to end against tmux 3.6a: `choose-tree` evaluates `-F` per
row in that row's context, so `#{?#{@cm_paused},…}` resolves per session.
The `(N)` shortcut key and the `name:` label are drawn by `choose-tree`
itself, not `-F`, so the session name always shows (identity intact) and
the badge sits beside it.

### Switcher integration

The binding is **installed at runtime** by the skill, not added to
`tmux/.tmux.conf` — the feature must travel with the skill, not depend on
a hand-edited config:

```bash
tmux bind-key w choose-tree -Zw -F '<format>'
```

Installed idempotently by the manager on invocation **and** by the pause
skill whenever a pause is set, so the badge renders whenever a paused
session exists, with or without a manager running.

Consequences, accepted as the price of keeping it skill-owned:

- The rebind is global and live. It is purely additive — identical to
  stock `choose-tree -Zw` except for the badge on paused sessions.
- It reverts to whatever `~/.tmux.conf` sets on a tmux **server restart**,
  and re-installs on the next manager invocation or pause. Between a
  server restart and the next manager activity, badges do not render.
- If the user later customises their own `w` binding, the runtime install
  overrides it; fold the `-F` in or accept the override.

The `-F` format applies to window and pane rows too (not just sessions),
so it must guard on node type — render the badge line only for sessions,
and reproduce the stock-ish line for windows/panes:

```
#{?session_format,
   #{?#{@cm_paused},⏸ paused · ,}#{session_windows} windows#{?session_attached,\, attached,},
   #{?window_format,#{window_index}: #{window_name}#{window_flags},#{pane_current_command}}}
```

Illustrative; exact format finalised in implementation. The non-paused
session branch reproduces the useful default bits (window count, attached
marker) so active lines do not look bare.

## Command surface

One worker-side skill, `claude-manager-pause`, a thin wrapper. It
**toggles**: active → pause, paused → unpause. Its description advertises
"pause / unpause / resume" phrasings so either verb routes to it. A
single toggle keeps one code path and avoids minting a
`/claude-manager-resume` skill that would overload shutdown's cold-resume
("resume via the manager", `claude --resume`).

Mechanics live in a new `## Pause` section in `claude-manager/SKILL.md`
as the single source of truth (mirroring how shutdown/wrap mechanics live
in the manager skill and the worker FLOW points to them). The worker
skill points to it. Manager-initiated pause/unpause is prose in the same
section — no separate manager skill.

Pause is light: no snapshot, no journal, no kill. It is a registry rewrite
under the lock (set/clear `paused`) plus the `@cm_paused` option and the
idempotent binding install.

## Return to active

- **Explicit** (reliable): worker `/claude-manager-pause` toggle, or the
  user tells the manager "unpause eng-1234". Clears `paused` and
  `@cm_paused`.
- **Auto-clear** (best-effort, conservative): during a reconcile or
  idle-detection pass, if a paused session's primary Claude pane reads
  **busy** (`esc to interrupt` / spinner — the existing idle-detection
  heuristic), the manager clears `paused` and `@cm_paused`. Busy is
  unambiguous activity; idle-at-prompt is **not** treated as activity (a
  parked session sits idle, so idle must not auto-unpause it). Reuses
  existing machinery; invents no new detection.

## Other interactions

- **Reconcile**: a paused session is still active (has `tmux_session`), so
  reconcile treats it as alive and preserves `paused`. The auto-clear
  check piggybacks on the existing reconcile / idle-detection pass.
- **Manager task list**: paused sessions show a `[paused]` prefix in the
  visible task list, alongside the existing `[active]` convention.
- **Shutdown / wrap of a paused session**: the rewrite drops `paused`
  with `tmux_session`; `@cm_paused` dies with the killed session.

## Files touched

- `claude-manager/SKILL.md` — new `## Pause` section; Registry edits
  (`paused` field, lifecycle-encoding line, worker-writable list); a line
  in Reconcile / idle-detection for auto-clear; binding install on
  invocation; `[paused]` task-list prefix; drop `paused` in the
  manager-side Shutdown and Wrap rewrites.
- `claude-manager-pause/SKILL.md` — new thin wrapper (toggle).
- `claude-manager-end/FLOW.md` — drop `paused` in the shutdown/wrap
  rewrites (one line each).

No `tmux/.tmux.conf` change — the binding is installed at runtime.

## Relationship to the `-end` naming follow-up

Pause does not kill tmux, so a worker `/claude-manager-pause` sits
**outside** the `-end` family by definition — `-end` is the
tmux-killing transitions (shutdown, wrap). This sharpens, rather than
complicates, the separate "Manager-exit vs worker -end naming clash"
follow-up; that item stays out of scope here.

## Out of scope

- **Demotion / sort-to-bottom.** `choose-tree` can only sort by index,
  name, or time, never by a flag, so reordering paused sessions to the
  bottom requires either renaming the session (rejected — mutates the
  primary key) or a custom switcher. If badges alone prove insufficient
  at higher session counts, demotion becomes its own follow-up via a
  `display-popup` + `fzf` switcher driven off the registry — never via
  name-mutation.

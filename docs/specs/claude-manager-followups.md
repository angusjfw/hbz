# Claude manager follow-ups

Ongoing list of known issues and pending work for the `claude-manager`
skill and its `-wrap` / `-shutdown` / `-end` siblings. Companions to
`2026-04-29-claude-manager-workflow.md` and the
`2026-05-22-claude-manager-sessions-pivot.md` follow-up spec. One
heading per item, terse context only — fixes and scoping decided when
picked up. Items below are ordered by priority, high to low.

## Manager-exit vs worker "-end" naming clash

"End" is overloaded: the `claude-manager-end` dir holds the *worker*
shutdown/wrap shared flow (FLOW.md, itself not a skill — no SKILL.md),
while the *manager* close-out is only the "Ending the manager session"
section in `claude-manager/SKILL.md` with no command of its own.
Confusing when invoking.

Leading resolution (probably — not committed; the prefix is unsettled):
rename the whole family to a role-based scheme `amux-<role>-<action>`,
which dissolves the clash by disambiguating on role rather than on the
action word (no more juggling end/teardown/exit):

- `amux-manager` (coordinator) + a new `amux-manager-exit` command (the
  missing close-out entry point)
- `amux-worker-{shutdown,wrap,pause}`; shared-flow dir
  `amux-worker-teardown`

`amux` = agent + tmux — motivation is that nothing here is Claude-
specific and `claude-manager` is a long, Claude-only-sounding prefix.
Name still tentative.

Scope is two layers with very different risk:

- **User-facing** — ~92 `claude-manager` refs across skills + specs, 5
  skill dirs, the `~/.claude/skills` symlinks. Mechanical, no runtime
  risk.
- **Internal plumbing** — `~/.local/state/claude-manager/` (registry,
  snapshots, resume), `@cm_paused`, watch PID naming. Renaming needs a
  *live* migration: at last check there was a running watch (manager in
  tmux `0:1.0`) and live registry sessions, so a careless `mv` orphans
  them and breaks in-flight workers (renamed commands stop resolving).
  Do it when the system is quiescent, or migrate the dir + restart the
  watch + have the running manager re-read.

Related gap (pending regardless of the rename): the manager exit flow's
step 5 is just "Exit" and says nothing about the manager's own tmux
session. Workers kill their tmux on shutdown/wrap; the manager can't
blanket-kill — it often runs in the user's primary/attached session
(e.g. default `0`), where killing tmux would drop the user to a bare
shell. The flow should state the rule: kill a dedicated manager session,
leave a shared/primary one.

## Demote paused sessions in the switcher

The paused state itself shipped (see
`2026-06-07-claude-manager-paused-state.md`): a `paused` registry field,
a `@cm_paused` tmux option, and a `⏸ paused` badge in `prefix+w`. What's
deferred is *demotion* — sorting parked sessions to the bottom of the
switcher rather than just badging them in place.

`choose-tree`'s only ordering control is `-O index|name|time`; there's
no per-session custom sort key, and the badge approach deliberately
leaves the session name unchanged (it stays `== registry id`), so the
name-prefix sort hack is out. Demotion therefore needs a custom popup
switcher: `display-popup` + `fzf` over `tmux list-sessions`, where
ordering and the paused marker are both trivial and driven off the
registry's `paused` flag. Only worth it if badges prove insufficient at
higher session counts.

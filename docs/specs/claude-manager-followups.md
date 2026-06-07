# Claude manager follow-ups

Ongoing list of known issues and pending work for the `claude-manager`
skill and its `-wrap` / `-shutdown` / `-end` siblings. Companions to
`2026-04-29-claude-manager-workflow.md` and the
`2026-05-22-claude-manager-sessions-pivot.md` follow-up spec. One
heading per item, terse context only — fixes and scoping decided when
picked up. Items below are ordered by priority, high to low.

## Manager-exit vs worker "-end" naming clash

The coordinator close-out is only a section in `claude-manager/SKILL.md`
("Ending the manager session") with no command of its own, while the
`claude-manager-end` skill dir actually holds the *worker* shutdown/wrap
shared flow (FLOW.md). "End" is overloaded across the two and it reads as
confusing when invoking. Candidate: name the manager close-out "exit"
(`/claude-manager-exit`?) — but confirm it's clearly distinct from the
worker shutdown/wrap and from the `-end` shared-flow dir before settling.
Decide: rename the worker `-end` dir, give the manager exit its own named
entry point, or both.

Related gap: the exit flow's step 5 is just "Exit" and says nothing about
the manager's own tmux session. Workers kill their tmux on wrap/shutdown;
the manager exit doesn't, so the asymmetry is confusing. It also can't
blanket-kill: the manager often runs in the user's primary/attached
session (e.g. default session `0`), where killing tmux would drop the
user to a bare shell. The flow should state the rule explicitly — kill a
dedicated manager session, leave a shared/primary one — rather than
leaving it to "Exit".

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

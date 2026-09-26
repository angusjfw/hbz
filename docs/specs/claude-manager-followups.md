# Claude manager follow-ups

Ongoing list of known issues and pending work for the `claude-manager`
skill and its `-wrap` / `-shutdown` / `-pause` siblings. Companions to
`2026-04-29-claude-manager-workflow.md` and the
`2026-05-22-claude-manager-sessions-pivot.md` follow-up spec. One
heading per item, terse context only — fixes and scoping decided when
picked up. Items below are ordered by priority, high to low.

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

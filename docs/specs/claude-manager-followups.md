# Claude manager follow-ups

Ongoing list of known issues and pending work for the `claude-manager`
skill and its `-wrap` / `-shutdown` / `-end` siblings. Companions to
`2026-04-29-claude-manager-workflow.md` and the
`2026-05-22-claude-manager-sessions-pivot.md` follow-up spec. One
heading per item, terse context only — fixes and scoping decided when
picked up. Items below are ordered by priority, high to low.

## Paused state for sessions

A third lifecycle state between active and shutdown: paused. The tmux
session stays alive and the worker keeps its state; the manager just
flags it so it sorts to the bottom of the tmux leader+w switcher and
shows a paused marker alongside existing identifiers in the title.
Use case: sessions waiting on review or other external state — the
switcher should distinguish "what I'm working on now" from "what's
parked for later" without killing tmux. Distinct from the retired
park concept, which moved sessions out of the manager's window list;
this one just relabels and reorders.

Switcher feasibility (deferred): leader+w is stock `choose-tree -Zw` —
no custom binding in the tmux config. `choose-tree`'s only ordering
control is `-O index|name|time`; there's no per-session custom sort
key, so "sort paused to the bottom" isn't natively simple. Two routes:
(a) rename paused sessions with a sort-affecting prefix/marker — but
that collides with the `tmux_session == registry id` convention; or
(b) replace `choose-tree` with a custom popup switcher (`display-popup`
+ `fzf` over `tmux list-sessions`) where ordering and a paused marker
are both trivial and can be driven off the registry's paused flag
rather than the session name. (b) is the cleaner path and the reason
this is more than a relabel.

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

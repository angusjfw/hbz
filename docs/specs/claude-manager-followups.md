# Claude manager follow-ups

Ongoing list of known issues and pending work for the `claude-manager`
skill and its `-wrap` / `-shutdown` / `-end` siblings. Companions to
`2026-04-29-claude-manager-workflow.md` and the
`2026-05-22-claude-manager-sessions-pivot.md` follow-up spec. One
heading per item, terse context only — fixes and scoping decided when
picked up. Items below are ordered by priority, high to low.

## Workers reach outside their own session for panes

A worker's free space is its own tmux session (Spawn step 4), but when
one needs a pane mid-work — run a server, open an editor, show output —
it sometimes scans the whole server (`tmux list-panes -a`) and reuses,
splits, sends keys to, or kills a pane in another session: the
manager's, another worker's, or the user's. Cross-session pane ops
trample the other occupant. Fix: workers scope every pane operation to
their own session (`-t <own-session>`); `-a` is for read-only layout
awareness, not a target list. Open question is the fix location — the
spawn brief (against the narrow-brief rule), the worker-side skills
(only loaded at wrap/shutdown, too late for normal work), or the global
tmux rulebook (right reach since every worker loads it, but it also
governs unmanaged sessions and currently recommends `list-panes -a`).

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

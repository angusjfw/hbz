---
name: claude-manager-pause
description: Toggle the paused flag on a claude-manager session from inside the worker pane — marks the registry entry parked (or clears it) and sets a tmux switcher badge, without killing tmux or the worker. Use when a session is waiting on review or other external state and you want it easy to skip in the prefix+w switcher, or to bring a parked session back. The manager observes via its watch.
---

# claude-manager-pause

`/claude-manager-pause`

Pause = flag this session as parked (waiting on review or other external
state) so it's easy to skip in the `prefix+w` switcher and the manager's
task list. Unlike shutdown and wrap, pause kills nothing — the tmux
session and worker stay alive. It toggles: pause an active session,
unpause a parked one.

This is a thin wrapper. The mechanics live in `claude-manager/SKILL.md`
§ Pause as the single source of truth. Read that section and run it for
this session.

From the worker's own pane:

1. Resolve this session and find its entry — the same preamble the
   shutdown/wrap flow uses (`claude-manager-end/FLOW.md` § Common
   preamble): `src_session=$(tmux display-message -p -t "$TMUX_PANE" '#S')`,
   then match `$src_session` against each entry's `tmux_session`.
2. Branch on whether the entry already has a `paused` field, then run
   the Pause or Unpause steps from `claude-manager/SKILL.md` § Pause —
   registry rewrite under the lock, the `@cm_paused` tmux option, the
   switcher binding, and the `[active]`/`[paused]` task-list prefix.

Vocabulary: "pause", "pause it", "park it" pause; "unpause", "resume it"
clear it. The manager picks up the change through its registry watch; if
no manager is running, the next invocation reconciles it.

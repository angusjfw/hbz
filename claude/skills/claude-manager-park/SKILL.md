---
name: claude-manager-park
description: Park a Claude session from inside the worker pane by asking the manager Claude to do it. Locates the manager pane via the registry header, sanity-checks it, and prompts it to perform the park. Use when in a worker pane and you want to park yourself without switching to the manager.
---

# claude-manager-park

In the claude-manager pattern, a manager Claude conversation oversees
parallel worker sessions in tmux. Parking moves a worker's tmux window
into a standalone tmux session — out of the manager's window list —
preserving it without shutting it down. This skill is the worker side:
prompt the manager to perform the park, since the manager owns
move-window and the registry.

A worker can park its own session without leaving its pane: locate the
manager Claude and prompt it to do the parking. The manager owns the
move-window mechanics and the registry edit; the worker's job is just
to make the request.

The manager records its tmux pane on the registry header
(`manager: <tmux-session>:<window>.<pane>`), refreshed on every
manager action. That's the primary way to find it. If the line is
absent or stale, fall back to scanning candidate panes in the worker's
tmux session for a Claude TUI.

Full registry shape and lifecycle is documented in
`docs/specs/2026-04-29-claude-manager-workflow.md`.

Unparking is not in scope here. The parked session is no longer in
the manager's window list, and you wouldn't run this from the parked
session anyway. Ask the manager when you want to bring it back.

## Park

`/claude-manager-park [<new-tmux-session-name>]`

Default target name is the worker's window name (which is the session
id, by manager convention).

1. Resolve the worker's location and target name:

   ```bash
   pane="$TMUX_PANE"
   src_session=$(tmux display-message -p -t "$pane" '#S')
   src_window=$(tmux display-message -p -t "$pane" '#I')
   src_pane=$(tmux display-message -p -t "$pane" '#P')
   window_name=$(tmux display-message -p -t "$pane" '#W')
   target="${1:-$window_name}"
   ```

2. Locate the manager pane. Read the registry header for `manager:`
   lines; pick the one whose tmux session matches `$src_session`:

   ```bash
   registry=~/.local/state/claude-manager/sessions.md
   mgr_target=$(awk '/^## / {exit} /^manager:/ {print $2}' "$registry" \
     | awk -F: -v s="$src_session" '$1 == s {print; exit}')
   ```

   If empty, scan candidate panes in the worker's tmux session and
   pick one with a Claude TUI signature (`> ` prompt, `esc to
   interrupt`, or a spinner). Try the lowest-numbered window's pane 0
   first (manager convention), then other panes in the same session.
   Skip the worker's own pane.

   ```bash
   for cand in $(tmux list-panes -s -t "$src_session" \
       -F '#{session_name}:#{window_index}.#{pane_index}' \
       | grep -v "^${src_session}:${src_window}\.${src_pane}$"); do
     if tmux capture-pane -p -J -t "$cand" -S -20 \
          | grep -qE '(^> |esc to interrupt)'; then
       mgr_target="$cand"; break
     fi
   done
   ```

   Bail with evidence if nothing matches.

3. Sanity-check the chosen pane before sending. Capture and confirm a
   Claude TUI signature even when the candidate came from the
   registry — the line may be stale:

   ```bash
   tmux capture-pane -p -J -t "$mgr_target" -S -10
   ```

   If the capture doesn't look like a Claude TUI, surface it and
   stop.

4. Send the park request. Submit with `Escape` then a brief sleep
   then `Enter` so it lands in both vim editorMode and non-vim:

   ```bash
   message="Please park my session. I'm in window $src_window ('$window_name'), pane $src_pane, of tmux session '$src_session'. Target tmux session name: '$target'."

   tmux send-keys -t "$mgr_target" -l -- "$message"
   tmux send-keys -t "$mgr_target" Escape
   sleep 0.3
   tmux send-keys -t "$mgr_target" Enter
   ```

5. Capture the manager pane after sending and surface the tail to the
   user as evidence the prompt landed. Don't poll for completion —
   the manager may be busy, and the registry write is eventual.

6. Tell the user the request was sent and how to attach to the parked
   session once the manager processes it
   (`tmux attach -t <target>`).

## What the manager does

The "Moving a session" flow on the manager side covers the mechanics:
`tmux new-session -d` placeholder, `tmux move-window -d`, drop the
placeholder, then update the registry to swap `tmux_window` for
`tmux_session` and stamp `last_touched`. The worker's request is just
the trigger; this skill carries no registry mutation of its own.

## Failure modes

- **No manager line and no Claude TUI in the worker's tmux session.**
  No manager to ask. Surface to the user; either start a manager
  elsewhere or park manually.
- **Manager line points at a pane that no longer looks like Claude.**
  Stale entry. Surface it; fall through to the scan; bail if scan
  fails too.
- **Multiple manager lines for the same tmux session.** Pick the
  first match; document the choice when surfacing evidence.

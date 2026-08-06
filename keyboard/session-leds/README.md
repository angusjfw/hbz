## 💡 Session LEDs

Claude Code session status on the Voyager's agent layer — one key per
session, Codex-Micro style. Toggle the agent layer (bottom-right key)
to see every session's state; press a session's key to switch the tmux
client to it. Spec and design decisions:
`docs/specs/2026-08-02-agent-session-leds.md`, and
`docs/specs/2026-08-04-direct-hid-renderer.md` for the daemon.

##### 🎨 States
| State | Colour | Source |
|-------|--------|--------|
| idle | white | SessionStart, or done + focused |
| working | blue | UserPromptSubmit |
| done | green | Stop (sticky until focused) |
| needs input | yellow | Notification |
| error | red | Claude died uncleanly; its tmux session lives on |
| off | unlit (grey in HUD) | Claude exited cleanly; slot stays bound to the tmux session |

Slots belong to tmux sessions, not Claude processes — a Claude restart
keeps its key, and `slots.json` remembers name→slot so a recreated
session reclaims its key when free — a live incumbent always wins.
The claude-manager plays no part in slots; assignment is the CLI's
alone, under a store lock. `agent-status slot <session> <n>` pins.
With several Claudes in one session, the entry shows the
highest-priority state among them (needs_input > error > working >
done > idle) and parks `off` when the last one leaves. Idle
"waiting for your input" notifications don't count as needs_input;
permission requests do. Labels prefer the tmux session name (manager sessions
are named descriptively), falling back to the cwd basename for
auto-numbered sessions. State for a tmux session that no longer
exists is dropped silently (killing a scratch session isn't an
error).

##### 🧩 Pieces
- `bin/agent-status` — state store CLI. Claude Code hooks pipe every
  event to `agent-status event`; state lands in
  `~/.local/state/agent-status/<tmux-session>.json`. Slots (key
  positions 1–36) are assigned here alone, under a store lock, with
  `slots.json` remembering name→slot across session lifetimes.
- `agent-deck/` — the renderer, and the whole feature besides: one Rust
  daemon talking raw HID to the board, with nothing GUI-resident in the
  path. It pairs once, follows the layer events the board pushes,
  re-reads the store when it changes (so nothing polls), diff-paints
  the slot LEDs, handles agent-layer key presses by position, and draws
  the HUD and toasts itself. Statuses show on the base layer (home
  markers repainted alongside) and on the agent layer, where the toggle
  key lights white; other layers keep their firmware colours. Slots 1–18
  are the right-hand letter rows (slot 1 = Y, LED 26), 19–36 spill onto
  the left half. Survives keyboard disconnects.

##### 📋 Requirements
- Hooks wired in Claude settings (`make ai`) and the status CLI
  symlinked (`make session-leds`, included in `make common`)
- A rust toolchain (Brewfile) to build `agent-deck`
- Keymapp (Brewfile) only to flash firmware, and only while the daemon
  is paused — nothing needs it running otherwise

New machine: `make common agent-deck-daemon`, flash the firmware. No
accessibility or automation permissions needed — the board is the input
device, so nothing taps the OS keyboard.

##### 🚀 Run
`make agent-deck-daemon` builds it, installs the launchd agent and
starts it (macOS; logs to `~/.local/state/agent-status/agent-deck.log`),
or run `agent-deck` directly in a pane. `agent-status list` shows
tracked sessions; `set`/`slot`/`clear` for manual control.

`agent-deck pause` goes fully silent and closes the HID device, which
is what frees the board for flashing firmware; `pause notify` stops
only the transition flashes; `base off` disables the always-on
base-layer display; `resume` / `status` round it out. Housekeeping
keeps running while paused.

The daemon needs the HID interface to itself, and Keymapp claims it
exclusively — so quit Keymapp before starting agent-deck, and pause
agent-deck before reaching for Keymapp.

Pressing a session key switches the most recently active tmux client to
that slot's session, brings the terminal forward and dismisses the agent
layer. The press arrives as a key position over HID, so no keycode
reaches the OS and no global hotkey is involved. The pressed key blinks
back, dimly when no session sits behind it.

##### 🖥️ Desktop
While the agent layer is on, agent-deck shows a heads-up display of
sessions with labels and states, in tmux creation order, refreshed live.
On a state change worth noticing (done, needs input, error) a toast
appears bottom-right; on layers with no status display the slot's LED
also flashes for ~1s (display only — key mapping is unaffected). Both
panels are transparent, click-through and never take focus.
`agent-deck preview` puts them on screen without the keyboard, for when
the styling is being worked on.

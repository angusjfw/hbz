## 💡 Session LEDs

Claude Code session status on the Voyager's agent layer — one key per
session, Codex-Micro style. Toggle the agent layer (bottom-right key)
to see every session's state; press a session's key to switch the tmux
client to it (adapter pending). Spec and design decisions:
`docs/specs/2026-08-02-agent-session-leds.md`.

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
  positions 1–18) come from the claude-manager registry `slot` field
  when present, else lowest-free auto-assignment.
- `bin/agent-leds` — renderer daemon. Polls Keymapp for the active
  layer and diff-paints slot LEDs 26–43 (right-hand rows, slot 1 = Y
  position). Statuses show on the base layer by default (home markers
  repainted alongside; `agent-leds base off` to disable) and on the
  agent layer, where the toggle key lights white. Other layers keep
  their firmware colours. Survives keyboard disconnects.
- `agent-deck/` — the whole feature as one Rust daemon talking raw HID
  to the board, no Keymapp, kontroll or Hammerspoon in the path. It
  pairs once, follows the layer events the board pushes, re-reads the
  store when it changes (so nothing polls), paints the LEDs, handles
  agent-layer key presses by position, and draws the HUD and toasts
  itself. Same controls as `agent-leds` (`pause [notify]`, `resume`,
  `base on|off`, `status`) on its own marker files. Slots 19–36 spill
  over onto the left half's letter rows.
  Spec: `docs/specs/2026-08-04-direct-hid-renderer.md`.

##### 📋 Requirements
- Hooks wired in Claude settings (`make ai`) and tools symlinked
  (`make session-leds`, included in `make common`)
- A rust toolchain (Brewfile) for `agent-deck`
- Keymapp (Brewfile) for flashing firmware — with agent-deck paused
- The `agent-leds` path additionally wants Keymapp ≥ 1.3.2 resident with
  its API enabled (`make keymapp-api`) and
  [kontroll](https://github.com/zsa/kontroll) on PATH (`make
  session-leds` fetches it on macOS)

New machine: `make common agent-deck-daemon`, flash the firmware. No
accessibility or automation permissions needed — the board is the input
device, so nothing taps the OS keyboard.

##### 🚀 Run
`make session-leds-daemon` installs and starts the launchd agent
(macOS; logs to `~/.local/state/agent-status/agent-leds.log`), or run
`agent-leds` directly in a pane. `agent-status list` shows tracked
sessions; `set`/`clear` for manual control.

`agent-leds pause` goes fully silent — no output and no Keymapp API
traffic (required while flashing firmware); `agent-leds pause notify`
stops only the transition flashes; `agent-leds base off` disables the
always-on base-layer display; `resume` / `status` round it out.
Housekeeping keeps running while paused.

`make agent-deck-daemon` is the same for `agent-deck` (logs to
`agent-deck.log`) and stops the `agent-leds` unit on the way, since the
two can't share the board. Controls are identical. It needs the HID
interface to itself: Keymapp holds the device exclusively, so quit
Keymapp before starting agent-deck, and `agent-deck pause` hands the
board back for flashing.

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

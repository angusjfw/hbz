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
| error | red | Claude process gone but its tmux session lives on |

State for a tmux session that no longer exists is dropped silently
(killing a scratch session isn't an error).

##### 🧩 Pieces
- `bin/agent-status` — state store CLI. Claude Code hooks pipe every
  event to `agent-status event`; state lands in
  `~/.local/state/agent-status/<tmux-session>.json`. Slots (key
  positions 1–18) come from the claude-manager registry `slot` field
  when present, else lowest-free auto-assignment.
- `bin/agent-leds` — renderer daemon. Polls Keymapp for the active
  layer; while the agent layer is toggled it takes host RGB control
  and diff-paints slot LEDs 26–43 (right-hand rows, slot 1 = Y
  position); restores firmware colours on leaving the layer. Survives
  keyboard disconnects.

##### 📋 Requirements
- Keymapp ≥ 1.3.2 (Brewfile) with its API enabled — `make keymapp-api`
  flips the config; the daemon launches Keymapp itself when needed
- [kontroll](https://github.com/zsa/kontroll) on PATH (`make
  session-leds` downloads it on macOS)
- Hooks wired in Claude settings (`make ai`) and tools symlinked
  (`make session-leds`, included in `make common`)

New machine: `make common keymapp-api session-leds-daemon hammerspoon`,
flash the firmware, grant Hammerspoon Accessibility.

##### 🚀 Run
`make session-leds-daemon` installs and starts the launchd agent
(macOS; logs to `~/.local/state/agent-status/agent-leds.log`), or run
`agent-leds` directly in a pane. `agent-status list` shows tracked
sessions; `set`/`clear` for manual control.

Pressing a session key sends Hyper+letter; the Hammerspoon config
(`hammerspoon/`, `make hammerspoon`) switches the most recently active
tmux client to that slot's session, focuses the terminal, and dismisses
the agent layer (`kontroll set-layer 0`).

##### 🖥️ Desktop
While the agent layer is on, Hammerspoon shows a heads-up display of
sessions with labels and states (driven by the daemon via
`hammerspoon://agent-hud`). On a state change worth noticing (done,
needs input, error) while on other layers, the daemon flashes the
slot's LED for ~1s (whole-board takeover, display only — key mapping is
unaffected) and Hammerspoon shows a matching toast bottom-right.

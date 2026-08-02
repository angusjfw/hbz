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
| error | red | tmux session died without SessionEnd |

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
- Keymapp ≥ 1.3.2 running with its API enabled (`api_enabled` in the
  `config` table of Keymapp's sqlite, plus `startup_autoconnect`)
- [kontroll](https://github.com/zsa/kontroll) on PATH
- Hooks wired in Claude settings (`make ai`) and tools symlinked
  (`make session-leds`, included in `make common`)

##### 🚀 Run
`agent-leds` in the foreground (tmux pane) or background. `agent-status
list` shows tracked sessions; `set`/`clear` for manual control.

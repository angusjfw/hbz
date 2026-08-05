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
keeps its key. Registry slots are authoritative: they evict
auto-assigned squatters, and registry sessions that haven't fired a
hook yet are seeded as `off` entries so reserved keys never look free.
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

`agent-leds pause` goes fully silent — no output and no Keymapp API
traffic (required while flashing firmware); `agent-leds pause notify`
stops only the transition flashes; `agent-leds base off` disables the
always-on base-layer display; `resume` / `status` round it out.
Housekeeping keeps running while paused.

Pressing a session key sends Hyper+letter; the Hammerspoon config
(`hammerspoon/`, `make hammerspoon`) switches the most recently active
tmux client to that slot's session, focuses the terminal, and dismisses
the agent layer (`kontroll set-layer 0`).

##### 🖥️ Desktop
While the agent layer is on, Hammerspoon shows a heads-up display of
sessions with labels and states (the daemon toggles a marker file that
a pathwatcher picks up, which also live-refreshes the HUD). On a state change worth noticing (done,
needs input, error), Hammerspoon shows a toast bottom-right; on layers
with no status display the daemon also flashes the slot's LED for ~1s
(display only — key mapping is unaffected).

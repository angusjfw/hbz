## 💡 Session LEDs

Claude Code session status on the Voyager — one key per session,
Codex-Micro style, lit on the base layer so the board is a glance at
what every session is doing. **Option-Space** summons the same list on
screen as a switcher: type a session's key to switch the tmux client to
it. The board's bottom-right key sends that chord, so it's one switcher
whether you reach for the keyboard or the shortcut. Spec and design
decisions: `docs/specs/2026-08-02-agent-session-leds.md`, and
`docs/specs/2026-08-04-direct-hid-renderer.md` for the daemon.

##### 🎨 States
| State | Colour | Source |
|-------|--------|--------|
| idle | white | SessionStart, or done + focused |
| working | blue | UserPromptSubmit |
| done | green | Stop (sticky until focused) |
| needs input | yellow | Notification |
| error | red | Claude died uncleanly; its tmux session lives on |
| off | unlit (grey on screen) | Claude exited cleanly; slot stays bound to the tmux session |

Slots belong to tmux sessions, not Claude processes — a Claude restart
keeps its key, and `slots.json` remembers name→slot so a recreated
session reclaims its key when free — a live incumbent always wins.
Memory lasts a day, enough to cover a reboot; `agent-status park
<session>` vouches for one that is coming back and holds its key for a
week, `clear <session>` releases it at once. Assignment is the CLI's
alone, under a store lock, and it never reads anything outside its own
store — the claude-manager parks what it shuts down and clears what it
wraps, because only it knows which a dead session was, but the CLI
works the same with no manager in sight.
`agent-status slot <session> <n>` pins.
With several Claudes in one session, the entry shows the
highest-priority state among them (needs_input > error > working >
done > idle) and parks `off` when the last one leaves. Tracking is one
Claude per pane, newest report wins — a nested `claude -p` inherits the
pane it was spawned from, so it shows there while it runs and the
pane's own session takes it back on its next event. Idle
"waiting for your input" notifications don't count as needs_input;
permission requests do. Labels prefer the tmux session name (manager sessions
are named descriptively), falling back to the cwd basename for
auto-numbered sessions. State for a tmux session that no longer
exists is dropped silently (killing a scratch session isn't an
error).

##### 🧩 Pieces
- `bin/agent-status` — the state store, and its only writer. Claude Code
  hooks pipe every event to `agent-status event`; state lands in
  `~/.local/state/agent-status/<tmux-session>.json`. Slots are assigned
  here alone, under a store lock, with `slots.json` remembering
  name→slot across session lifetimes. The renderer asks for the three
  transitions no hook can report — `set <session> error`, `demote` for a
  `done` the user has now looked at, `drop` for a session that has gone
  away (which keeps the slot memory, where `clear` releases it).
- `agent-deck/` — the renderer and the switcher: one Rust daemon talking
  raw HID to the board, with nothing GUI-resident in the path. It pairs
  once, follows the layer events the board pushes, re-reads the store
  when it changes (so nothing polls), diff-paints the slot LEDs, and
  draws the switcher and toasts itself. Statuses show on the base layer,
  home markers repainted alongside; every other layer keeps its firmware
  colours. Slots 1–17 are the right-hand letter rows (slot 1 = Y, LED
  26), 18–33 spill onto the left half. The modifier keys in among them
  are skipped: the switcher is typed into, so a key you can't type is a
  slot you can't reach. Survives keyboard disconnects.

##### 📋 Requirements
- Hooks wired in Claude settings (`make ai`) and the status CLI
  symlinked (`make session-leds`, included in `make common`)
- A rust toolchain (Brewfile) to build `agent-deck`
- Keymapp (Brewfile) only to flash firmware, and only while the daemon
  is paused — nothing needs it running otherwise

New machine: `make common agent-deck-daemon`, flash the firmware. No
accessibility or automation permissions needed — Option-Space is one
registered chord, not a tap, and nothing reads the OS keyboard.

##### 🚀 Run
`make agent-deck-daemon` builds it, installs the launchd agent and
starts it (macOS; logs to `~/.local/state/agent-status/agent-deck.log`),
or run `agent-deck` directly in a pane. `agent-status list` shows
tracked sessions; `set`/`slot`/`clear` for manual control.

`agent-deck pause` goes fully silent and closes the HID device, which
is what frees the board for flashing firmware; `resume` and `status`
round it out. Housekeeping keeps running while paused.

The daemon needs the HID interface to itself, and Keymapp claims it
exclusively — so quit Keymapp before starting agent-deck, and pause
agent-deck before reaching for Keymapp.

With the daemon down the bottom-right key types an Option-Space
non-breaking space into whatever has focus, which is easy to miss in a
prompt or a commit message.

##### 🖥️ Desktop
The switcher lists every tracked session with its label, state and key,
in tmux creation order, refreshed live. Type a session's key — the same
letter its board key uses — or pick with ↑↓ and Enter; Escape, the
chord again, or clicking away dismisses it and hands focus back. It's
the only time the overlay takes key focus.

Switching moves the most recently active tmux client and brings the
terminal forward. tmux names a client by its tty and has no client id,
so a suspended client on the same tty — invisible to `list-clients`,
but still first in the name lookup — can swallow the switch silently;
every switch is confirmed against the client tmux lists, and a
suspended one found in the way is killed so the retry lands. If it
still won't move, a toast says so rather than leaving it looking like a
slow terminal.

On a state change worth noticing (done, needs input, error) a toast
appears bottom-right. Toasts are transparent, click-through and never
take focus. `agent-deck preview` puts the panels on screen from the
live store, for when the styling is being worked on.

# Agent session LEDs

Physical status board for Claude Code sessions on the ZSA Voyager, modelled
on the [Codex Micro](https://worklouder.cc/codex-micro): each managed session
gets a key whose LED shows its state, and pressing the key switches the tmux
client to that session. An on-screen twin renders the same state with labels.

## States

| State | Colour | Trigger |
|-------|--------|---------|
| idle | white | session registered, nothing running (SessionStart, post-attach) |
| working | blue | prompt submitted, agent busy (UserPromptSubmit) |
| done | green | turn finished (Stop) |
| needs input | yellow | permission request / waiting notification (Notification) |
| error | red | Claude process vanished from a live tmux session (pane command check) |
| off | unlit | Claude exited cleanly; slot stays bound while the tmux session lives |

Slots bind to tmux sessions (the switch target), not Claude processes:
SessionEnd parks the entry as `off` rather than freeing the slot, so
Claude restarts don't shuffle keys. Only tmux-session death (GC) or
manual `clear` frees a slot.

Multi-Claude sessions have a durable owner: the first Claude claims
the entry (session_id + pane_id recorded); events from other Claudes
are ignored while the owner's pane is still a live Claude, and
takeover is allowed once the entry is parked or the owner's pane
stops being one. The owner's SessionEnd parks the entry only when no
other Claude pane remains — otherwise ownership is released for a
survivor to claim on its next event.

`done` is sticky until the session is focused or a new prompt starts, then
falls back to `idle`.

## Architecture

State flows one way: hooks → status CLI → state store → renderers.
The state store is the contract; everything downstream is replaceable.

### Status CLI (portable)

Single entry point, e.g. `agent-status`. Subcommands:

- `event` — invoked by Claude Code hooks; reads hook JSON on stdin,
  maps hook event → state, writes the store. All hooks point here;
  they stay one-liners in `settings.json`.
- `set <session> <state>` — manual override.
- `list` / `clear [<session>]` — inspection and cleanup.

Store: `~/.local/state/agent-status/<session>.json` with
`{state, slot, tmux_session, label, session_id, ts}`; renderers
re-read it on their poll cycle. Python stdlib only — must run on
macOS, Linux, WSL.

### Slot mapping (claude-manager integration)

18 slots — the three right-hand letter rows, slot 1 = Y position
(LED 26) through slot 18 (LED 43). New optional registry field
`slot: <1..N>` on session entries: manager assigns the lowest free
slot on spawn, clears it on wrap/shutdown (pause keeps it). The
status CLI prefers the registry slot; sessions without one (including
unmanaged) get the lowest free slot auto-assigned on first event.

### LED renderer (portable core)

Daemon that watches the store and paints keys via
[kontroll](https://github.com/zsa/kontroll) → Keymapp API
(v1.3.2+, API enabled; Unix socket on macOS/Linux, TCP port on
Windows). `set-rgb <led-index>` per slot, `restore-rgb-leds` on exit.

Host RGB control is all-or-nothing: while active the firmware paints
nothing (`rawhid_state.rgb_control`) — no layer colours, frozen board.
The renderer owns the board on the base layer (statuses always
visible; the base ledmap's home markers are repainted alongside —
`agent-leds base off` reverts to firmware colours) and on the agent
layer (statuses + toggle key white). Any other layer releases control
so firmware colours show. It polls `GetStatus` (~100ms, 15ms
round-trip) for `current_layer` and must survive keyboard disconnects
(boards re-enumerate; reconnect via `GetKeyboards`/`ConnectAny`).
`agent-leds pause` stops all API traffic (needed while flashing
firmware); `pause notify` stops only transition flashes.

### Input adapter (per-platform, thin)

Agent-layer keys send Hyper+A…Hyper+R (Ctrl+Alt+Shift+GUI): F13–F24
only covers 12 keys, Hyper combos scale to all 18 and dodge the Linux
F13+ keysym quirk. Adapter maps Hyper+<letter> → slot → focus terminal +
`tmux switch-client -t <tmux_session>` (target resolved from the store
by slot). Per platform:

- macOS: Hammerspoon (also hosts the on-screen twin)
- Linux/sway: `bindsym` in existing sway config
- WSL: Windows-side hotkey tool; later

Terminal emulator is irrelevant to switching (tmux does it); the
adapter only needs to focus the terminal app, and only the global-
hotkey route is terminal-agnostic — which is why keys are bound in the
adapter, not in tmux.

### On-screen twin

Same store, rendered as a labelled grid (session id, ticket, state).
v1: Hammerspoon canvas on macOS. The store contract keeps the door
open for a cross-platform mini-app later.

## Keyboard layout change

Repurpose layer 3 (numpad — unused) as the **agent layer**:

- Keep `TG(3)` on the bottom-right key as the toggle.
- Right-hand letter rows: session keys 1–18 sending Hyper+A…R
  (row-major: Y position = slot 1 = Hyper+A).
- Clear the layer's per-key LED colours — a dark layer means nothing
  flashes in the poll gap before the renderer takes host control.
- Always-visible base-layer indicators are out for v1: host control
  freezes the whole board (see spike findings). Revisit via firmware
  raw HID if wanted.

Done in `keyboard/voyager/src` directly (Oryx can't import source);
build and flash per `keyboard/README.md`.

## Repo layout

- `keyboard/session-leds/` — status CLI, renderer daemon, launchd plist
- `hammerspoon/` — macOS adapter (symlinked as `~/.hammerspoon`)
- `claude/settings.json.example` — hook wiring (all events →
  `agent-status event`)
- `claude/skills/claude-manager/SKILL.md` — slot field + assign/clear
  steps (optional: everything works without the manager, slots are
  just auto-assigned instead of stable)

## Spike (before building)

1. kontroll paint semantics: does `set-rgb` on one LED blank the rest;
   can the renderer repaint layer colours + overlays without flicker;
   latency; behaviour across layer switches; `restore-rgb-leds`.
2. Empirical LED index map for the Voyager (52 LEDs).
3. Confirm F13–F24 assignable in Oryx.
4. Keymapp as a resident dependency: autostart, auto-connect,
   coexistence with normal use.
5. `tmux switch-client` with multiple attached clients.
6. Error-state detection options (manager watch on pane death vs
   hooks).

### Findings

- **Setup**: Keymapp installs via `brew install --cask keymapp`
  (1.3.7). The API toggle lives in the `config` table of
  `~/Library/Application Support/.keymapp/keymapp.sqlite3`
  (`api_enabled`, `startup_autoconnect`) — flip with sqlite3 while
  Keymapp is stopped, no GUI needed. Socket appears at
  `~/Library/Application Support/.keymapp/keymapp.sock`.
- **kontroll**: no brew package; universal macOS binary from GitHub
  releases → `~/.local/bin`. Its default socket path assumes a
  sandboxed Keymapp (`~/Library/Containers/io.zsa.keymapp/…`), which
  the cask install isn't — every call needs
  `-p "$HOME/Library/Application Support/.keymapp/keymapp.sock"`.
  The renderer should wrap this.
- **Paint semantics** (item 1): `set-rgb` engages host control for
  the whole board — unpainted LEDs go dark and firmware layer colours
  freeze entirely until `restore-rgb-leds`. ~6ms per LED, restore is
  instant and clean. `--sustain <ms>` auto-reverts a paint. So: no
  passive overlay on the base layer; the renderer owns the board only
  while the agent layer is toggled. An always-visible base-layer
  aggregate indicator is out for v1 (would need firmware raw HID).
- **LED index map** (item 2): authoritative from `rgb_matrix.layout`
  in ZSA's QMK fork (`keyboards/zsa/voyager/keyboard.json`), verified
  on hardware. Row-major, left half 0–25 (thumbs 24–25), right half
  26–51 (thumbs 50–51). Right top row Y..\\ = 26–31.
- **Layer tracking**: `GetStatus.current_layer` reports MO holds and
  TG toggles reliably (100ms poll caught 1s holds; round-trip 15ms).
  The automouse layer (4) does not show up via the API — fine, only
  layer 3 matters. Connections drop when the board re-enumerates;
  the daemon needs a reconnect loop (`startup_autoconnect` only
  applies at Keymapp launch).
- **F13–F24** (item 3): assignable in Oryx. Linux caveat: some map to
  `XF86*`/`NoSymbol` keysyms by default; fix is the
  `fkeys:basic_13-24` XKB option
  (https://schnouki.net/post/2024/tip-f13-f24-keys-with-zsa-keyboards-on-linux/).
  macOS sees them as plain F-keys, bindable in Hammerspoon.
- **tmux** (item 5): `switch-client` works from outside tmux but the
  client must be explicit: `switch-client -c <client_tty> -t <session>`.
  With several clients the adapter should pick the most recently
  active (`list-clients -F '#{client_tty} #{client_activity}'`).
- **Errors** (item 6): hooks can't report a crashed process. Cheapest
  honest signal: renderer/manager watch marks a slot red when its
  tmux session or Claude process dies without a clean `SessionEnd`.
  Implemented in the renderer: a gone tmux session is GC'd silently
  (killing a scratch session isn't a crash); a live session whose
  Claude pane vanished goes red. Claude panes are recognised by
  `pane_current_command` — the CLI reports its version string (e.g.
  `2.1.220`), with claude/node accepted as fallbacks. Host-side
  features (toasts, GC, done-demotion) run even with the keyboard
  disconnected; only LED work needs the connection.

## Plan

1. ~~Spike~~ — findings above.
2. ~~Status CLI + hook wiring~~ — `keyboard/session-leds/bin/agent-status`,
   hooks in `claude/settings.json.example`. Verified live: hook events
   were picked up without restarting running sessions, and a real
   Notification painted its key yellow mid-test.
3. ~~LED renderer~~ — `keyboard/session-leds/bin/agent-leds`.
   Diff-paints concurrently; done→idle demotion on focus and
   dead-session→error detection live here.
4. ~~Agent layer~~ — done in source (`keyboard/voyager/src`), not Oryx:
   Oryx can't import source, so the repo keymap is now the source of
   truth and `make firmware` builds it against ZSA's QMK fork. The
   navigator/scroll keycodes moved to the `zsa/navigator_trackball`
   module. Home board flashed and verified end-to-end (LEDs + physical
   key switching); work board pending. Gotcha: stop the agent-leds
   daemon before flashing — its API connection blocks Keymapp's flash.
5. ~~Hammerspoon adapter~~ — `hammerspoon/agent-sessions.lua`, verified
   with physical keys. Uses hs.task (hs.execute's login-shell wrapper
   mangles quoting). Self-dismisses the agent layer after a switch via
   `kontroll set-layer -i 0`.
6. ~~Canvas twin~~ — HUD grid (labels + states) while the agent layer
   is on, plus bottom-right toasts on done/needs-input/error; both
   driven by the daemon through `hammerspoon://` URL events. The same
   transitions flash the slot LED ~1s from other layers (whole-board
   takeover, display-only, then restored).
7. ~~claude-manager slots~~ — spawn/cold-resume assign, shutdown/wrap
   drop (SKILL.md + end FLOW.md). The CLI re-reads the registry slot
   on every event since the manager's registry write lands after the
   worker's first hooks. Keymapp is now self-serve too:
   `make keymapp-api` flips the sqlite config, and the daemon launches
   Keymapp itself when the socket is missing.
8. ~~Daemon supervision~~ — launchd agent, `make session-leds-daemon`.
   No manager coupling: the daemon and CLI are standalone; the manager
   only ever contributes `slot` fields via the registry.
9. Later: Linux/sway adapter; WSL; aggregate base-layer indicator;
   direct-HID renderer — speak the Oryx protocol (open source in the
   zsa modules: `ORYX_SET_RGB_LED`, `ORYX_RGB_CONTROL`,
   `ORYX_EVT_LAYER` pushes layer changes) via hidapi, dropping the
   resident Keymapp dependency (kept only for flashing) and replacing
   layer polling with push events. Needs the pairing handshake
   (`ORYX_CMD_PAIRING_INIT/VALIDATE`) implemented.

## Non-goals (v1)

- Command keys beyond switch-to-session (accept/reject/push-to-talk)
- Standalone GUI app
- WSL support

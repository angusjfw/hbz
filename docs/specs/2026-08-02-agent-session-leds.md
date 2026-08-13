# Agent session LEDs

Physical status board for Claude Code sessions on the ZSA Voyager, modelled
on the [Codex Micro](https://worklouder.cc/codex-micro): each managed session
gets a key whose LED shows its state, and pressing the key switches the tmux
client to that session. An on-screen twin renders the same state with labels.

## States

| State | Colour | Trigger |
|-------|--------|---------|
| idle | white | session registered, nothing running (SessionStart, post-attach) |
| working | blue | prompt submitted or tool completed (UserPromptSubmit, PostToolUse — the latter clears stale needs_input after a permission is answered, since no hook fires for resume) |
| done | green | turn finished (Stop) |
| needs input | yellow | permission request (Notification; idle "waiting for your input" notices are ignored) |
| error | red | Claude process vanished from a live tmux session (pane command check) |
| off | unlit | Claude exited cleanly; slot stays bound while the tmux session lives |

Slots bind to tmux sessions (the switch target), not Claude processes:
SessionEnd parks the entry as `off` rather than freeing the slot, so
Claude restarts don't shuffle keys. Only tmux-session death (GC) or
manual `clear` frees a slot.

Multi-Claude sessions aggregate: the entry tracks per-Claude states
(`claudes: {session_id: {state, pane_id}}`) and shows the
highest-priority one (needs_input > error > working > done > idle).
A Claude's exit removes its sub-state; the entry parks `off` only
when the last Claude pane is gone. Claudes whose pane stops being one
are pruned on later events.

Notification hooks are classified by message: permission requests map
to needs_input; Claude Code's idle "waiting for your input"
notifications are ignored (a finished session is `done`/`idle`, not
demanding attention).

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

### Slot mapping (single mechanism, remembered)

36 slots — right-hand letter rows 1–18 (slot 1 = Y, LED 26), left
rows 19–36 as spillover. Assignment lives in the status CLI alone,
serialised by a store lock; the claude-manager knows nothing about
slots (an earlier registry-slot design created two competing
assigners and evicted long-running sessions — removed).

Persistence is a `slots.json` name→slot memory beside the state
files: a recreated session (same tmux name) reclaims its key when
free; a live incumbent always wins. New sessions fill the right half
before spilling left — within each half they prefer never-remembered
slots, then take over the stalest memory, so a dead session's
reservation never pushes a new session onto spillover. Dead sessions
drop from the live store immediately. `agent-status slot <session> <n>`
pins manually; `clear` forgets.

Memory has two tiers, because nothing in tmux tells a session that is
coming back from one that is simply gone. A name is remembered for a
day — enough to cover a reboot — and `park <session>` vouches for one
that will return, holding its key for a week. Whoever knows the
difference says so at the transition: the claude-manager parks a
session it shuts down and clears one it wraps. The verbs are generic
and this store never reads the registry, so slots work the same for a
session no manager has ever heard of; they just age out in a day.

Memory is deliberately not liveness-gated. A live tmux session holds
its key through its own store entry (parked `off` when Claude exits),
so memory is the only thing that carries a key across the session's
lifetime; forgetting every name without a live session would empty it
out, and would fire on the first hook after a boot — when nothing has
started yet — handing keys out by whichever session raced first. The
two tiers get the same result the honest way: an unvouched name simply
expires.

### LED renderer (portable core)

`agent-deck` watches the store and writes LEDs over the board's raw
HID interface directly — no broker process (see the direct-HID spec).

Host RGB control is all-or-nothing: while active the firmware paints
nothing (`rawhid_state.rgb_control`) — no layer colours, frozen board.
The renderer owns the board on the base layer (statuses always
visible; the base ledmap's home markers are repainted alongside —
`agent-deck base off` reverts to firmware colours) and on the agent
layer (statuses + toggle key white). Any other layer hands control
back so firmware colours show. Layer changes and keypresses are
pushed by the board, so nothing polls, and it survives disconnects
(boards re-enumerate; it reopens on a retry). `agent-deck pause`
closes the device (needed while flashing firmware); `pause notify`
stops only transition flashes.

### Input adapter (per-platform, thin)

Agent-layer keys send nothing at all: the daemon resolves the physical
key position → slot → focus terminal + `tmux switch-client -t
<tmux_session>` (target resolved from the store by slot). Per
platform:

- macOS and Linux: none for the board — `agent-deck` reads key
  positions straight from the board over raw HID (see the direct-HID
  spec), so nothing taps the OS keyboard. One global chord
  (Option-Space, a Carbon hot key — not an event tap, no accessibility
  permission) summons the on-screen switcher for when the board isn't
  plugged in.
- WSL: a Windows-side HID bridge; later

Terminal emulator is irrelevant to switching (tmux does it); the
daemon only needs to focus the terminal app. Binding in the daemon
rather than in tmux is what makes it work from anywhere, not just from
inside a terminal.

### On-screen twin

Same store, rendered as a labelled grid (session id, ticket, state),
drawn by `agent-deck` itself as a transparent click-through overlay
(eframe), on every platform it runs on. Option-Space opens the same
grid as a keyboard switcher — the no-board path: it takes key focus,
a session's own key label (or arrows + Enter) switches to it, and
Escape or focus loss dismisses.

## Keyboard layout change

Repurpose layer 3 (numpad — unused) as the **agent layer**:

- Keep `TG(3)` on the bottom-right key as the toggle.
- Right-hand letter rows: session keys 1–18 (row-major: Y position =
  slot 1), sending `KC_NO` — the daemon reads their positions.
- Clear the layer's per-key LED colours — a dark layer means nothing
  shows before the renderer takes host control.
- Always-visible base-layer indicators are out for v1: host control
  freezes the whole board (see spike findings). Revisit via firmware
  raw HID if wanted.

Done in `keyboard/voyager/src` directly (Oryx can't import source);
build and flash per `keyboard/README.md`.

## Repo layout

- `keyboard/session-leds/` — status CLI, the `agent-deck` daemon (Rust
  crate), launchd plist
- `claude/settings.json.example` — hook wiring (all events →
  `agent-status event`)

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
9. ~~Direct-HID renderer~~ — `agent-deck` took over painting, input,
   the HUD and toasts, and the python daemon, kontroll, resident
   Keymapp and Hammerspoon are all gone with it. Spec:
   `docs/specs/2026-08-04-direct-hid-renderer.md`.
10. Later: Linux/sway; WSL; aggregate base-layer indicator.

## Non-goals (v1)

- Command keys beyond switch-to-session (accept/reject/push-to-talk)
- Standalone GUI app
- WSL support

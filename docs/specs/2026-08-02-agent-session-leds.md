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
| error | red | session crashed or hook-reported failure (detection TBD — spike) |
| off | unlit | slot unassigned |

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
- `set <session> <state>` — manual/manager override.
- `list` / `watch` — for renderers and debugging.

Store: `~/.local/state/agent-status/<session>.json` with
`{state, slot, tmux_session, label, ts}`. File mtime is the change
signal (fswatch/inotify). Shell or python, no exotic deps — must run
on macOS, Linux, WSL.

### Slot mapping (claude-manager integration)

New optional registry field `slot: <1..N>` on session entries.
Manager assigns the lowest free slot on spawn, clears it on wrap/
shutdown (pause keeps it). The status CLI reads slot + tmux_session
from the registry; sessions without a slot get state tracking but no
key. Unmanaged sessions: out of scope for v1.

### LED renderer (portable core)

Daemon that watches the store and paints keys via
[kontroll](https://github.com/zsa/kontroll) → Keymapp API
(v1.3.2+, API enabled; Unix socket on macOS/Linux, TCP port on
Windows). `set-rgb <led-index>` per slot, `restore-rgb-leds` on exit.

Known constraint: host RGB control takes over the whole board and the
firmware's per-layer colours yield (`rawhid_state.rgb_control`). The
renderer therefore either (a) paints only while the agent layer is
active, or (b) repaints the active layer's ledmap colours itself plus
the status keys. Spike decides.

### Input adapter (per-platform, thin)

Agent-layer keys send F13+. Adapter maps F13+n → focus terminal +
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
- Right-hand main block: session keys 1–N sending F13+.
- LEDs on this layer default dark in the ledmap; the renderer paints
  status colours.
- Maybe later: one always-visible aggregate indicator on the base
  layer (e.g. yellow if any session needs input) — depends on spike
  finding (b) viable.

Edit in Oryx, re-export to `keyboard/voyager/`, flash (per
`keyboard/README.md` flow).

## Repo layout

- `keyboard/session-leds/` — renderer daemon, input adapter config,
  twin, status CLI
- `claude/settings.json.example` — hook wiring (all events →
  `agent-status event`)
- `claude/skills/claude-manager/SKILL.md` — slot field + assign/clear
  steps

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
- **API**: handshake works (Keymapp 1.3.7 / Kontroll 1.0.4).
  LED paint semantics and index map (items 1–2) still open — need the
  Voyager plugged in.
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
  Deferred to the manager-integration milestone.

## Plan

1. Spike (above) — outcome recorded back into this spec.
2. Status CLI + hook wiring; verify states change during real sessions.
3. LED renderer painting slots from the store.
4. Oryx layout change (agent layer), flash, end-to-end LED test.
5. Hammerspoon adapter (F13+n switching) + canvas twin.
6. claude-manager slot assignment in the skill.
7. Later: Linux/sway adapter; WSL; aggregate base-layer indicator.

## Non-goals (v1)

- Unmanaged (non-registry) session tracking
- Command keys beyond switch-to-session (accept/reject/push-to-talk)
- Standalone GUI app
- WSL support

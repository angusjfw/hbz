# Direct-HID session LEDs

Replace the Keymapp + kontroll + Hammerspoon-hotkey stack under the
session LEDs (`docs/specs/2026-08-02-agent-session-leds.md`) with one
daemon that speaks the Oryx raw-HID protocol to the Voyager directly.
Keymapp is then needed only to flash firmware; nothing GUI-resident is
load-bearing.

## Why

- **No resident Keymapp**: today's daemon breaks (and must self-heal)
  whenever Keymapp quits, crashes, or is "tidied away". Direct HID
  removes the dependency and the whole relaunch/reconnect class of
  problems.
- **Push, not poll**: the board pushes layer changes and keypresses;
  today we poll `GetStatus` at 100ms through two IPC hops. LED paints
  and HUD response become event-driven.
- **The keyboard becomes the input device for its own feature**:
  `ORYX_EVT_KEYDOWN` carries the physical key position, so the daemon
  handles agent-layer presses itself — no global hotkeys, no Hyper
  keycodes reaching the OS, no Hammerspoon in the input path, and the
  same code path on every platform.

## Protocol (from ZSA module source, `modules/zsa/oryx/`)

Verified against `oryx.h` / `oryx.c` at zsa/qmk_firmware `firmware25`
(protocol version 0x05). Raw HID frames of `RAW_EPSIZE` bytes
(QMK raw HID endpoint, usage page 0xFF60 / usage 0x61); byte 0 is the
command/event code, params follow, `0xFE` stop bit.

Commands we need:

- `ORYX_CMD_PAIRING_INIT` — replies `EVT_PAIRING_SUCCESS`
  unconditionally and starts the event stream (also emits the current
  layer). No challenge; `PAIRING_VALIDATE` is a backwards-compat no-op.
- `ORYX_SET_RGB_LED (led, r, g, b)` — writes one LED into the
  firmware's `webhid_leds` buffer (plain RGB, no HSV dance) and
  auto-engages host control (a custom rgb_matrix effect renders the
  buffer; firmware layer colours stop).
- `ORYX_RGB_CONTROL (0|1)` — engage/release host control; release
  reloads firmware colours from eeprom.
- `ORYX_SET_LAYER (on, layer)` — `layer_move`; replaces
  `kontroll set-layer` for self-dismiss.
- `ORYX_CMD_GET_FW_VERSION` — identity check.

Events while paired:

- `ORYX_EVT_LAYER (layer)` — every layer change (sent once on pairing
  too). Replaces layer polling.
- `ORYX_EVT_KEYDOWN / KEYUP (col, row)` — every physical keypress,
  regardless of keycode. Agent-layer handling: on layer 3, map
  (col,row) → slot 1–18 or the toggle key.
- `ORYX_EVT_RGB_CONTROL (bool)` — control-state acknowledgements.

## Architecture

Unchanged: hooks, `agent-status`, the state store, slot semantics,
claude-manager integration — the store remains the contract.

`agent-leds` v2 (still python; + `hidapi` via brew/pip):

- Owns the HID connection: enumerate by VID/PID + usage page, open,
  pair, read events on a thread; reconnect on unplug (hidapi
  enumeration poll, keyboard-swap friendly).
- LED painting as today (frames, diff), but via `SET_RGB_LED` writes —
  event-driven repaint on layer events and state-store changes.
- **Input**: `EVT_KEYDOWN` on the agent layer → resolve slot → tmux
  switch (most-recent client, as now) → focus terminal
  (`open -a` on macOS, `swaymsg` on Linux) → `SET_LAYER 0` dismiss.
- Pause: `pause` closes the HID device entirely (flash-safe — Keymapp
  needs exclusive access), `pause notify` as today.

### Presentation: one cross-platform renderer

The HUD is a first-class part of the feature (labels are what the
LEDs can't show), so it gets one implementation, not per-platform
adapters: a small python renderer process (`agent-hud`) that owns
both the HUD panel and toasts, reading the same state dir and
marker-file contract as today. Retires Hammerspoon entirely.

- **Windowing: Tkinter** (stdlib, no new deps) — undecorated,
  always-on-top panels on macOS/Linux/Windows. Drawing code kept
  separate from the windowing layer so a later upgrade (Qt) is a
  swap, not a rewrite. Cosmetic limits accepted for now (rounded
  corners macOS-only).
- **Wayland/sway**: clients can't self-position; placement comes from
  a `for_window` rule keyed on the app id in the sway config.
- Toasts render in the same process (small transient panels,
  bottom-right), replacing both the Hammerspoon canvases and any
  notification-center fallback.
- Supervised like the daemon (launchd/systemd user unit), or spawned
  by it.

## Firmware follow-up

Once daemon input handling is proven, the agent layer's Hyper+A…R
keycodes can become no-ops (position events fire regardless), removing
the last OS-visible side effect. Keep them until the KC_NO event spike
passes (below).

## Spikes

1. hidapi on macOS: open the Voyager's 0xFF60 interface, pair, observe
   `EVT_LAYER`/`EVT_KEYDOWN` — confirm no TCC/Input-Monitoring prompt
   and no fight with normal typing.
2. Frame details: RAW_EPSIZE (32 vs 64), report-id prefix handling on
   each platform.
3. `SET_RGB_LED` brightness behaviour vs today's kontroll path (the
   webhid effect applies raw RGB; check perceived brightness).
4. Keypress events for `KC_NO`/`KC_TRANSPARENT` keys (enables dropping
   Hyper keycodes).
5. Exclusive access: confirm Keymapp and the daemon can't both hold
   the interface, and that `pause` (device closed) is sufficient for
   flashing.
6. Reconnect across keyboard swaps between machines (re-enumerate +
   re-pair loop).

## Plan

1. Spike script proving pair + events + LED write (single file,
   throwaway).
2. `agent-leds` v2 behind a flag (`AGENT_LEDS_HID=1`), old path kept
   until parity: paint/flash/HUD-marker/pause/GC/demote.
3. Move switching into the daemon; build the `agent-hud` renderer
   (Tkinter HUD + toasts); retire Hammerspoon (hotkeys, canvases and
   the config symlink).
4. Firmware: agent-layer keycodes to no-ops (post spike 4); reflash.
5. Remove kontroll/Keymapp-API path, keymapp-api make target becomes
   flash-only doc; update specs/READMEs.
6. Linux/sway and WSL follow the same daemon (WSL: a thin Windows-side
   HID bridge is still required — the device can't be split between
   Windows and WSL; design there when tackled).

## Non-goals

- Replacing Keymapp for flashing.
- Status LED (the little indicator LEDs) control.
- Bluetooth/wireless boards.

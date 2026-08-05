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

**`agent-deck`: one Rust binary** replacing `agent-leds`, kontroll,
resident Keymapp and Hammerspoon. Cargo crate at
`keyboard/session-leds/agent-deck/` (monorepo subproject — own build,
wired into the Makefile, `target/` gitignored, binary installed to
`~/.local/bin`).

- **HID** (`hidapi` crate — same as kontroll uses): enumerate by
  VID/PID + usage page 0xFF60, open, pair, read the event stream;
  reconnect on unplug (keyboard-swap friendly).
- **LEDs**: frames + diffing as today, via `SET_RGB_LED` writes —
  repaint driven by layer events and state-store changes (`notify`
  crate on the state dir), no polling anywhere.
- **Input**: `EVT_KEYDOWN` on the agent layer → resolve slot → tmux
  switch (most-recent client, as now) → focus terminal
  (`open -a` on macOS, `swaymsg` on Linux) → `SET_LAYER 0` dismiss.
- **HUD + toasts in-process** (`egui`/eframe): GPU-rendered
  transparent, undecorated, always-on-top panels on all three
  platforms — the HUD is first-class (labels are what LEDs can't
  show). Wayland/sway can't self-position windows; placement comes
  from a `for_window` rule in the sway config.
- Pause: `pause` closes the HID device entirely (flash-safe — Keymapp
  needs exclusive access), `pause notify` as today. Control marker
  files unchanged.
- Supervised by launchd/systemd user unit, as today.

`agent-status` (hooks CLI) stays python for now: stdlib-only,
working, and its ~50ms interpreter startup sits at turn boundaries
where it's invisible. A Rust port behind the same store contract is
an easy later swap if hook latency ever matters.

Cost acknowledged: a Rust toolchain on each machine (brew/pacman
`rust`) and a compile step in `make`.

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

1. Spike script proving pair + events + LED write (python + hidapi,
   single file, throwaway — fastest way to de-risk the protocol
   before the Rust build).
2. `agent-deck` crate: HID + LED painting + state watching to parity
   with today's daemon (paint/flash/pause/GC/demote), running
   alongside the old stack until swapped.
3. Input handling + HUD/toasts in the same binary; retire Hammerspoon
   (hotkeys, canvases, config symlink) and the marker-file HUD hop.
4. Firmware: agent-layer keycodes to no-ops (post spike 4); reflash.
5. Remove kontroll/Keymapp-API path and the agent-leds python daemon;
   keymapp-api make target becomes flash-only doc; update
   specs/READMEs/Makefile (`make agent-deck` builds + installs).
6. Linux/sway and WSL follow the same binary (WSL: a thin
   Windows-side HID bridge is still required — the device can't be
   split between Windows and WSL; design there when tackled).

## Non-goals

- Replacing Keymapp for flashing.
- Status LED (the little indicator LEDs) control.
- Bluetooth/wireless boards.

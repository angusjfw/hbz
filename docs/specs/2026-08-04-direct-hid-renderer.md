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
- **36 slots**: since input is positional (no keycodes needed), the
  left-half letter rows (LEDs 0–17, same row-major order) become
  spillover slots 19–36, and their keycodes go with the right half's. Assignment always prefers 1–18; the left
  half only lights when the right is full. Home markers yield to
  occupied spillover slots as on the right.
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

## UI/UX: parity first, then deltas

The rebuild must not regress the lived-in UX. Parity checklist —
identical behaviour, byte-for-byte semantics:

- Base layer: always-on statuses + home markers (statuses outrank
  markers on shared LEDs); `base off` reverts to firmware colours.
- Agent layer: statuses, toggle key white, self-dismiss after switch.
- Other layers: firmware colours untouched; ~1.2s single flash +
  toast on done/needs-input/error; toast bottom-right, ~2.5s.
- HUD: top-centre, dark translucent rounded panel, dot/key/label/state
  rows, **tmux creation order** (session id — matches the switcher),
  live refresh, show/hide with the layer.
- Controls unchanged: `pause [notify]`, `resume`, `base on|off`,
  `status`, on `deck-*` marker files.
- Store semantics untouched (per-Claude state aggregation, park/off,
  GC, done-demotion, slot memory in the CLI (no registry involvement
  — reconcile and eviction are gone), notification
  classification).

Deliberate improvements:

- Push events end the poll gap: LEDs and HUD respond to a layer
  toggle in ~ms, no dark flash while the poll catches up.
- No OS-visible keycodes: agent-layer presses can't leak Hyper chords
  into apps (today they do if Hammerspoon is dead).
- Press feedback on the board, agent layer only (never on the base
  display): every press gets a brief pulse on that key; an empty key
  pulses subtly instead of raising an on-screen alert.
- 36 slots via left-half spillover (above).
- One process, one supervisor; the Keymapp relaunch dance is gone.

Design decisions (settled up front):

- **Visuals**: refresh allowed — same layout, information and feel
  (dark rounded panels), egui may improve typography/spacing where
  clearly better; no redesign.
- **Overlays are click-through**, HUD and toasts both — no mouse
  interactions, keyboard-first.
- **Toasts are first-class**: bottom-right, ~2.5s, on
  done/needs-input/error transitions while the agent layer isn't
  toggled; display-only.
- **Config = constants in code**, as today's scripts; a config file
  only if it ever hurts.
- **All press feedback confined to the toggled agent layer**; the
  base-layer display is pure status, never blinks.

Regressions to guard against:

- **Exclusive HID**: Keymapp can't connect while agent-deck holds the
  interface — today they coexist because Keymapp *is* the broker.
  Using Keymapp (flashing, live training) requires `agent-deck pause`
  (which closes the device). Acceptable; document it.
- **HUD latency**: create the egui window once and keep it hidden —
  cold window + GPU context init would be slower than today's
  pathwatcher. Show/hide must stay <50ms.
- **Focus stealing**: the overlay must never take key focus (macOS
  accessory activation policy; no activation on show).
- **Visual fidelity**: egui defaults look like a game UI, so the panels
  are styled to the canvases they replace (dark translucent slabs,
  dot/key/label/state rows). Its bundled font has no keycap glyphs
  (⇧ ⇥ ⌃) — a platform symbol font is loaded as a fallback.
- Toolchain cost per machine (rust via brew/pacman) — accepted with
  the monorepo convention.

## Firmware follow-up

Done: the agent layer's Hyper+A…R keycodes are `KC_NO`, so a session
press has no OS-visible side effect at all. Its left half follows —
those keys are spillover slots 19-36, and RGB, volume and media keycodes
there would fire alongside a switch — leaving layer 3 as nothing but the
status display. The RGB controls moved to layer 2.

## Spikes

1. ~~hidapi on macOS~~ — proven end to end (python + brew hidapi):
   Voyager at vid 0x3297 / pid 0x1977, usage_page 0xFF60 / usage 0x61;
   opened with **no TCC prompt**; `GET_FW_VERSION` round-trip;
   `PAIRING_INIT` → success + initial layer; `EVT_KEYDOWN/KEYUP`
   (col,row) streamed live during normal typing with zero interference;
   `SET_RGB_LED` accepted and `RGB_CONTROL 0` acked by event.
2. ~~Frame details~~ — command frame: byte 0 command, params after,
   32-byte reports, leading 0x00 report id on write (macOS).
3. `SET_RGB_LED` brightness vs kontroll path: paint accepted, and the
   webhid effect scales writes by the same global brightness the layer
   ledmaps use (`rgb_matrix_config.hsv.v`, `rgb_matrix_kb.inc`), so
   colours should match; visual check still pending.
4. ~~Keypress events for `KC_NO` keys~~ — passed on hardware: with the
   agent layer's letters flashed as `KC_NO`, presses still switch
   sessions and nothing reaches the focused app.
5. ~~Exclusive access / `pause` for flashing~~ — exclusivity is real
   and mutual: with Keymapp connected, opening fails with
   `exclusive access and device already open`, so Keymapp must be quit
   before starting agent-deck. The reverse works as designed —
   `agent-deck pause` closes the device and Keymapp was connected
   again within a second, so flashing stays available.
6. Reconnect: verified across `pause`/`resume` (device closed and
   reacquired within the retry interval); a physical unplug/replug
   still to try.

Privacy note from spike 1: a paired listener receives every keypress
position — effectively keystroke telemetry. agent-deck must never log
or persist key events beyond agent-layer handling.

## Plan

1. Spike script proving pair + events + LED write (python + hidapi,
   single file, throwaway — fastest way to de-risk the protocol
   before the Rust build).
2. ~~`agent-deck` crate~~ — `keyboard/session-leds/agent-deck/`, built
   and installed by `make agent-deck`. HID pairing, layer-event
   following, notify-driven store reads, diff paints, flashes, toasts,
   the pause/base controls and the housekeeping it owns (GC, persisted
   error conversion, done-demotion). Slots are the CLI's alone — the
   deck reads the `slot` on each entry and never assigns, reserves or
   remembers one. Verified live: connect, pause/resume around Keymapp,
   GC, error write-back, demotion, colour fidelity and the layer
   toggle. Nothing polls — the loop wakes on board events, store
   changes and a 1s housekeeping tick.
3. ~~Input handling + HUD/toasts in the same binary~~ — presses arrive
   as key positions and resolve through a matrix-to-LED table generated
   from ZSA's `keyboard.json`; agent-deck switches the tmux client,
   fronts the terminal and dismisses the layer over `SET_LAYER`. The
   HUD and toasts are eframe panels on one transparent, click-through,
   accessory-policy window, created hidden at startup and shown with
   the layer. Hammerspoon is gone — hotkeys, canvases, config symlink,
   cask and the marker-file HUD hop with it. `agent-deck preview` draws
   the panels without a board for styling work.
4. ~~Firmware: agent-layer keycodes to no-ops~~ — the right-hand
   session keys are flashed and verified; the build that blanks the
   left half too (and moves RGB to layer 2) is committed, awaiting a
   flash.
5. ~~Remove the kontroll/Keymapp-API path and the agent-leds python
   daemon~~ — script, launchd plist, `session-leds-daemon` and
   `keymapp-api` targets and the kontroll fetch are all deleted;
   `make session-leds` now just symlinks the status CLI, `make mac`
   builds the deck, and Keymapp is documented as a flashing tool that
   wants the daemon paused.
6. Linux/sway and WSL follow the same binary (WSL: a thin
   Windows-side HID bridge is still required — the device can't be
   split between Windows and WSL; design there when tackled).

## Non-goals

- Replacing Keymapp for flashing.
- Status LED (the little indicator LEDs) control.
- Bluetooth/wireless boards.

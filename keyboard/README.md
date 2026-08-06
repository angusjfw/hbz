## ⌨️ ZSA Voyager

Split ergonomic keyboard, per-key RGB, with the Navigator trackball attachment.
Layout "voyup", maintained in [Oryx](https://configure.zsa.io/voyager/layouts/PYnPm/latest/0).

##### 📁 Contents
- `voyager/src/` — QMK source (Oryx export + local agent-layer changes)
- `voyager/firmware/` — compiled firmware, flash with [Keymapp](https://www.zsa.io/flash)
- `session-leds/` — Claude session status on the agent layer (own README)

##### 🗺️ Layers
| # | Access | Purpose |
|---|--------|---------|
| 0 | base | QWERTY. Ctrl/Shift on left pinky column, Esc·GUI·Alt inner bottom row, Del·Bspc and arrows bottom right. Green home markers. |
| 1 | hold either inner thumb key | Number row, shifted symbols, brackets/braces. |
| 2 | hold bottom-left key | F1–F12, volume, RGB brightness/toggle, vim-style arrows on right home row, Caps Lock (red indicator while active). |
| 3 | toggle bottom-right key | Agent sessions, nothing else: every key sends `KC_NO` and agent-deck reads positions over raw HID (see `session-leds/`). Slots 1–18 on the right rows, 19–36 spilling onto the left. Only the toggle key and the right-hand edge stay live. |
| 4 | automatic on trackball motion | Mouse buttons, drag scroll, CPI up/down, layer lock. |
| 5 | hold second bottom-left key | Manual copy of the mouse layer. |

Each layer has its own per-key LED colour map (`ledmap` in `keymap.c`); layer
colours yield whenever the host takes RGB control over raw HID.

##### 🖱️ Trackball
Navigator pointing-device driver with high-res scroll.
Custom keycodes: `DRAG_SCROLL` (hold to scroll), `TOGGLE_SCROLL`,
`NAVIGATOR_INC_CPI`/`NAVIGATOR_DEC_CPI`.
`NAVIGATOR_TURBO` and `NAVIGATOR_AIM` are defined in `keymap.c` but not mapped.

##### ⚙️ Notable config
- `TAPPING_TERM 0` — no tap-hold behaviour anywhere; all layer keys are plain holds/toggles.
- Automouse: layer 4 activates after ~20 units of trackball movement, deactivates on timeout.
- Oryx module enabled (`ORYX_ENABLE`) — the raw HID protocol agent-deck
  speaks (pairing, layer events, key positions, LED writes) lives there.
- Most RGB matrix animations compiled out; static per-layer colours only.

##### 🔮 Future
- Automouse ↔ manual mouse layer handover works but is messy; revisit.

##### 🔄 Updating
Source of truth is `voyager/src/` in this repo — the agent layer (3) was
added locally and no longer matches the [Oryx layout](https://configure.zsa.io/voyager/layouts/PYnPm/latest/0),
which Oryx can't import back. For big layout edits: edit in Oryx,
re-export, re-apply the local delta (or adopt
[ZSA's Oryx+custom-QMK flow](https://blog.zsa.io/oryx-custom-qmk-features/)).

Build: `make firmware` (needs `brew install qmk/qmk/qmk` after trusting
the qmk/qmk, osx-cross/arm and osx-cross/avr taps, plus
[ZSA's QMK fork](https://github.com/zsa/qmk_firmware) cloned at
`~/dev/zsa-qmk` — override with `QMK_FORK=`). Flash the built bin from
`voyager/firmware/` with Keymapp — run `agent-deck pause` first, which
closes its HID connection so Keymapp can claim the board, then
`agent-deck resume` afterwards.

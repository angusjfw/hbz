## ⌨️ ZSA Voyager

Split ergonomic keyboard, per-key RGB, with the Navigator trackball attachment.
Layout "voyup", maintained in [Oryx](https://configure.zsa.io/voyager/layouts/PYnPm/latest/0).

##### 📁 Contents
- `voyager/src/` — QMK source exported from Oryx (`keymap.c`, `config.h`, `rules.mk`, `keymap.json`)
- `voyager/firmware/` — compiled firmware, flash with [Keymapp](https://www.zsa.io/flash)

##### 🗺️ Layers
| # | Access | Purpose |
|---|--------|---------|
| 0 | base | QWERTY. Ctrl/Shift on left pinky column, Esc·GUI·Alt inner bottom row, Del·Bspc and arrows bottom right. Green home markers. |
| 1 | hold either inner thumb key | Number row, shifted symbols, brackets/braces. |
| 2 | hold bottom-left key | F1–F12, volume, vim-style arrows on right home row, Caps Lock (red indicator while active). |
| 3 | toggle bottom-right key | Numpad, media transport, RGB controls. |
| 4 | automatic on trackball motion | Mouse buttons, drag scroll, CPI up/down, layer lock. |
| 5 | hold second bottom-left key | Manual copy of the mouse layer. |

Each layer has its own per-key LED colour map (`ledmap` in `keymap.c`); layer
colours yield whenever the host takes RGB control via Keymapp's API.

##### 🖱️ Trackball
Navigator pointing-device driver with high-res scroll.
Custom keycodes: `DRAG_SCROLL` (hold to scroll), `TOGGLE_SCROLL`,
`NAVIGATOR_INC_CPI`/`NAVIGATOR_DEC_CPI`.
`NAVIGATOR_TURBO` and `NAVIGATOR_AIM` are defined in `keymap.c` but not mapped.

##### ⚙️ Notable config
- `TAPPING_TERM 0` — no tap-hold behaviour anywhere; all layer keys are plain holds/toggles.
- Automouse: layer 4 activates after ~20 units of trackball movement, deactivates on timeout.
- Oryx module enabled (`ORYX_ENABLE`) — live training and host RGB control work.
- Most RGB matrix animations compiled out; static per-layer colours only.

##### 🔄 Updating
Source of truth is the Oryx layout. After editing there:
download the source zip and firmware, replace `voyager/src/` and
`voyager/firmware/`, flash with Keymapp, commit.

To build locally instead, use [ZSA's QMK fork](https://github.com/zsa/qmk_firmware):
copy `voyager/src/` into `keyboards/zsa/voyager/keymaps/<name>/` and `qmk compile`.

# Theme Design: acme-hbz for macOS

Spec for coordinating the acme-hbz light colour scheme across Ghostty, vim/nvim, and tmux.

## Philosophy

Colour as functional UI signal, not syntax decoration. Bold and italic over colour where the distinction is subtle. Terminal-first: Ghostty owns the palette, vim and tmux reference ANSI slots. Less is more — but not nothing.

Inspired by Acme and Plan 9 aesthetics, but this is a modern, functional scheme, not a historical recreation.

## Font

**JetBrains Mono** (Ghostty default), default size. Ligatures enabled.

Tried Fira Code Retina — legible but more decorative/exaggerated. JetBrains Mono is cleaner and more invisible, which fits the functional philosophy. Ghostty embeds it, so no install needed.

Other systems to sync:
- Konsole config (`konsole/hbz.profile`) currently set to Fira Code Medium 14pt
- Consider updating if Konsole is used again

## Palette

Warm cream background, dark grey foreground, muted earth-tone ANSI colours with light tints available for background shading.

### Colour definitions

| Name     | Light (tint)  | Mid           | Dark (saturated) |
|----------|---------------|---------------|------------------|
| White    | `#FFFFEC` W1  | `#EEEEA7` W2 | `#999957` W3     |
| Grey     |               |               | `#424242` W4     |
| Red      | `#F8E7E7` R1  | `#F2ACAA` R2 | `#B85C57` R3     |
| Green    | `#EFFEEC` G1  | `#98CE8F` G2 | `#57864E` G3     |
| Yellow   | `#EAEBDB` Y1  | `#B7B19C` Y2 | `#8F7634` Y3     |
| Blue     | `#E2F1F8` B1  | `#A6DCF8` B2 | `#2A8DC5` B3     |
| Magenta  |               | `#D0D0F7` M2 | `#8888C7` M3     |
| Cyan     | `#EEFEFF` C1  | `#B0ECED` C2 | `#6AA7A8` C3     |
| Accent   |               |               | `#030093` A1     |

### ANSI slot mapping (Ghostty)

| Slot | Name    | Normal (dark)    | Bright (light)   |
|------|---------|------------------|-------------------|
| 0    | black   | `#424242` W4     | `#999957` W3      |
| 1    | red     | `#B85C57` R3     | `#F2ACAA` R2      |
| 2    | green   | `#57864E` G3     | `#98CE8F` G2      |
| 3    | yellow  | `#8F7634` Y3     | `#B7B19C` Y2      |
| 4    | blue    | `#2A8DC5` B3     | `#A6DCF8` B2      |
| 5    | magenta | `#8888C7` M3     | `#D0D0F7` M2      |
| 6    | cyan    | `#6AA7A8` C3     | `#B0ECED` C2      |
| 7    | white   | `#EAEBDB` Y1     | `#FFFFEC` W1      |

Special:
- Background: `#FFFFF0` (ivory — slightly less yellow than W1, tuned by eye)
- Foreground: `#424242` (W4)
- Cursor: `#424242` (W4)
- Cursor text: `#FFFFF0`
- Selection background: `#EEEEA7` (W2)
- Selection foreground: `#424242` (W4)

## Ghostty

Custom theme file at `ghostty/acme-hbz` in the repo, symlinked to `~/.config/ghostty/themes/acme-hbz`.

Config (`ghostty/config`):
```
theme = acme-hbz
```

Font is Ghostty's default (JetBrains Mono) — no override needed.

## Vim / Neovim

Re-enable the existing `vim/acme-hbz.vim` colourscheme with targeted additions.

### Existing (keep as-is)

- Normal: W4 on W1 (dark grey on cream)
- Comments: green (G3), italic
- Visual selection: W4 on W2 (dark on gold)
- Search/IncSearch: W4 on W2, inverse for active match (magenta bg)
- StatusLine: W4 on C1 (cyan tint), bold+underline for active
- LineNr: W3 on Y1 (muted, italic)
- CursorLineNr: W1 on M3 (magenta accent)
- Pmenu: green tones (G1/G2/G3)
- Error/SpellBad: red (R3)
- Todo/MatchParen: W4 on W2
- Bold for ErrorMsg, ModeMsg, MoreMsg, WarningMsg, Directory

### Additions

**Diff highlight groups** (hex values from palette, not ANSI — approach B):
- DiffAdd: bg `#EFFEEC` (G1) — green tint
- DiffDelete: bg `#F8E7E7` (R1), fg `#B85C57` (R3) — red tint, red text
- DiffChange: bg `#EAEBDB` (Y1) — yellow/neutral tint
- DiffText: bg `#B7B19C` (Y2) — darker yellow tint for changed text within a changed line

**Minimal syntax additions** (bold/italic, no colour):
- Statement (keywords): bold
- String: italic
- All other syntax groups (Type, Identifier, Constant, PreProc, Special): unchanged (inherit fg)

## Tmux

Use ANSI colour names, no hex values. Terminal provides the actual colours.

### Status bar
- Default background (transparent to terminal)
- Foreground: dim text
- Active window: green (colour2) — functional signal
- Inactive windows: muted (colour8 / bright black)

### Pane borders
- Inactive: muted (default)
- Active: green or cyan — needs to be clearly distinguishable, especially between tmux panes and vim splits

### Notifications and messages
- Command/copy messages: bold, yellow fg (colour3)
- Activity/bell alerts: red fg (colour1)
- Mode indicator (prefix, copy mode): magenta bg (colour5)

## Makefile integration

New target `theme` (or extend existing targets):
- Symlink `ghostty/acme-hbz` to `~/.config/ghostty/themes/acme-hbz`
- Existing `vim` and `ghostty` targets already handle the config symlinks

## Cross-platform sync

The palette source of truth is `theme/palette`. Other platform configs translate it:
- `konsole/AcmeHbz.colorscheme` — already matches (uses original W1 `#FFFFEC` bg)
- `WindowsTerminal/settings.json` — already matches (uses original W1 `#FFFFEC` bg)

Todos:
- [ ] Update Konsole and Windows Terminal background to `#FFFFF0` when those systems are next touched
- [ ] Update Konsole font from Fira Code to JetBrains Mono
- [ ] Sync any future palette changes back to these files

No generator or shared format — the spec is the source of truth, synced manually.

## Iteration plan

This is a visual design — expect to iterate once we can see it. The process:
1. Create Ghostty theme file and apply it
2. Re-enable vim colourscheme and add diff/syntax groups
3. Update tmux colours
4. Review each tool visually and adjust

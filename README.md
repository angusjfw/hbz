## hbz
Minimal, AI-enabled, keyboard-driven dev setup.

##### 🔧 Setup
Symlinks config files into place via Makefile targets.
`make install` for Mac, `make arch` for Arch Linux, `make common` otherwise.
Platform differences handled with conditional sourcing.

##### 📦 What's here
- `zsh/`, `vim/`, `tmux/` — shared across platforms
- `agents/`, `claude/` — AI tool config, symlinked to `~/.claude/`
- `ghostty/`, `Brewfile` — Mac
- `sway/`, `konsole/`, `mako/` — Arch
- `arch-install-xps9370` — Arch install notes

##### 🌸 Themes
Acme-inspired light colour scheme across vim, tmux and terminal.
Fira fonts. Minimal wallpapers.

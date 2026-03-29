## hbz
Multiplatform, AI-enabled, keyboard-driven dev setup.

##### 🔧 Setup
Symlinks config files into place via Makefile targets.
`make install` for Mac, `make arch` for Arch Linux, `make wsl` for WSL, `make common` otherwise.
Platform differences handled with conditional sourcing.

##### 📦 What's here
- `zsh/`, `vim/`, `tmux/` — shared across platforms
- `agents/`, `claude/` — AI tool config, symlinked to `~/.claude/`
- `vscode/` — VS Code settings
- `ghostty/`, `Brewfile` — Mac
- `WindowsTerminal/` — WSL
- `sway/`, `konsole/`, `mako/` — Arch
- `arch-install-xps9370` — Arch install notes

##### 🌸 Themes
Acme-inspired light colour scheme across vim, tmux and terminal.
Fira fonts. Minimal wallpapers.

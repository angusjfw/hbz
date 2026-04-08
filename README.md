## hbz
Multiplatform, AI-enabled, keyboard-driven dev setup.

##### 🔧 Setup
Makefile symlinks config into place.

`make install` (Mac), `make arch`, `make wsl`, or `make common` for just
the shared config.

Platform targets run common setup plus extras;
differences handled with conditional sourcing in shared files.

##### 📦 Common
`zsh/`, `vim/`, `tmux/`, `git/`, `vscode/` — shared across platforms.

`agents/`, `claude/` — AI tool config, symlinked to `~/.claude/`.

##### 🍏 Mac
`ghostty/` terminal config. `brew/` for packages.

##### 🪟 WSL
`WindowsTerminal/` settings, auto-detected Windows username.

##### 🐧 Arch
`sway/`, `konsole/`, `mako/`, `wallpapers/`.
`arch/` — install notes. `pacman/` for package lists.

##### 🌸 Themes
Acme-inspired light colour scheme. See `theme/palette`.

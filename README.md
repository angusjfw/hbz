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
`ghostty/` terminal config. `Brewfile` for packages.

##### 🪟 WSL
`WindowsTerminal/` settings, auto-detected Windows username.

##### 🐧 Arch
`sway/`, `konsole/`, `mako/`, `wallpapers/`.
`arch-install-xps9370` — install notes. `pkglist.txt` for packages.

##### 🌸 Themes
Acme-inspired light colour scheme across vim, tmux and terminal.
Fira fonts. Minimal wallpapers.

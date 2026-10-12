## hbz
Multiplatform, AI-enabled, keyboard-driven dev setup.

##### 🔧 Setup
Makefile symlinks config into place.

`make install` (Mac), `make arch`, `make wsl`, `make rpi`, or `make common` for just
the shared config.

Platform targets run common setup plus extras;
differences handled with conditional sourcing in shared files.

##### 📦 Common
`zsh/`, `vim/`, `tmux/`, `git/`, `vscode/` — shared across platforms.

`agents/`, `claude/`, `pi/` — AI tool config, symlinked to `~/.claude/` and `~/.pi/agent/`. Skills in `agents/skills/` are shared by both.

##### 🍏 Mac
`ghostty/` terminal config. `brew/` for packages.

##### 🪟 WSL
`WindowsTerminal/` settings, auto-detected Windows username.

##### 🐧 Arch
`sway/`, `konsole/`, `mako/`, `wallpapers/`.
`arch/` — install notes. `pacman/` for package lists.

##### 🍓 Raspberry Pi
`rpi` — apt packages plus shell/editor/agent config for a headless Pi.
`apt/` for the package list.

##### ⌨️ Keyboard
`keyboard/` — ZSA Voyager layout (QMK source + firmware from Oryx).

##### 🌸 Themes
Acme-inspired light colour scheme. See `theme/palette`.

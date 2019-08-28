## Arch Linux Configuration for Dell XPS 13 9370
Minimal, keyboard-driven Arch Linux setup with [swaywm](http://swaywm.org/).

![screenshot](https://github.com/angusjfw/hbz/raw/master/screenshot-2018-04-16.png)

##### 🔧 OS installation
Mostly standard Arch install guide. Disk encryption with `dm-crypt`; separate
`/home` partition with `lvm`; boot with `sytemd-boot`.

##### 📦 Core software installation
Official packages installed from `pkglist.txt` which is generated with
`pacman -Qqen`; few extras from AUR and `npm`.

##### 🌸 Dotfile symlinking
Config for `zsh`, `vim`, `tmux`, `sway`, `konsole`, `base16` colours, `fira`
fonts & more...

### Usage
Install OS following instructions in `arch-install-xps9370`.  
Install the rest with `make install` or specific `make` targets.

Other suggestions:
- https://chrome.google.com/webstore/detail/material-simple-dark-grey/ookepigabmicjpgfnmncjiplegcacdbm

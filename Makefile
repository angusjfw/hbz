DIR=$(shell pwd)

install: pkg zsh vim tmux node dircolors fonts sway konsole mako

pkg:
	sudo pacman -Syu
	sudo pacman -S --needed - < pkglist.txt

zsh:
	ln -sf ${DIR}/zsh/.zshrc ~/.zshrc
	ln -sf ${DIR}/zsh/.zprofile ~/.zprofile
	ln -sf ${DIR}/zsh/.zsh-history-substring-search.zsh \
	  ~/.zsh-history-substring-search.zsh
	yaourt -S --noconfirm zsh-pure-prompt

vim:
	ln -sf ${DIR}/vim/.vimrc ~/.vimrc
	mkdir -p ~/.config/nvim
	ln -s /home/angus/hbz/vim/acme-hbz.vim /home/angus/.config/nvim/colors/acme-hbz.vim
	mkdir -p ~/.vim/colors
	ln -s /home/angus/hbz/vim/acme-hbz.vim /home/angus/.vim/colors/acme-hbz.vim
	ln -sf ${DIR}/vim/.vimrc ~/.config/nvim/.vimrc
	curl -fLo ~/.vim/autoload/plug.vim --create-dirs \
	  https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim
	curl -fLo ~/.local/share/nvim/site/autoload/plug.vim --create-dirs \
	  https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim

tmux:
	ln -sf ${DIR}/tmux/.tmux.conf ~/.tmux.conf

node:
	mkdir -p ~/.npm-global
	npm config set prefix '~/.npm-global'
	npm install --global n

dircolors:
	mkdir -p ~/.config/dircolors
	ln -sf ${DIR}/dircolors/config ~/.config/dircolors/config

fonts:
	yaourt -S --noconfirm ttf-fira-sans
	yaourt -S --noconfirm ttf-fira-mono
	yaourt -S --noconfirm ttf-fira-code

sway:
	mkdir -p ~/.config/sway
	ln -sf ${DIR}/sway/config ~/.config/sway/config
	ln -sf ${DIR}/sway/start-sway ~/start-sway

mako:
	mkdir -p ~/.config/mako
	ln -sf ${DIR}/mako/config ~/.config/mako/config

konsole:
	mkdir -p ~/.local/share/konsole
	ln -sf ${DIR}/konsole/konsolerc ~/.config/konsolerc
	ln -sf ${DIR}/konsole/DarkPastels.colorscheme \
	  ~/.local/share/konsole/DarkPastels.colorscheme
	ln -sf ${DIR}/konsole/AcmeHbz.colorscheme \
	  ~/.local/share/konsole/AcmeHbz.colorscheme
	ln -sf ${DIR}/konsole/hbz.profile ~/.local/share/konsole/hbz.profile

wallpapers:
	mkdir -p ~/.config/wallpapers
	ln -sf ${DIR}/wallpapers/darkgrey.png ~/.config/wallpapers/darkgrey.png
	ln -sf ${DIR}/wallpapers/white.png ~/.config/wallpapers/white.png

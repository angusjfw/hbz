DIR=$(shell pwd)

.PHONY: install mac arch common zsh vim nvim tmux ghostty ai brew brew-check z dircolors sway konsole mako wallpapers

install: mac

mac: brew common ghostty

arch: pkg common z dircolors sway mako konsole wallpapers

common: zsh vim nvim tmux ai

brew:
	brew bundle --file=${DIR}/Brewfile

brew-check:
	@echo "Installed but not in Brewfile:"
	@brew bundle cleanup --file=${DIR}/Brewfile 2>/dev/null | grep -v "^Would\|^Run\|^$$" || echo "  (none)"
	@echo "\nIn Brewfile but not installed:"
	@brew bundle check --file=${DIR}/Brewfile --verbose 2>/dev/null | grep "^→" | sed 's/→ /  /' || echo "  (none)"

pkg:
	sudo pacman -Syu
	sudo pacman -S --needed - < pkglist.txt

zsh:
	ln -sf ${DIR}/zsh/.zshrc ~/.zshrc
	ln -sf ${DIR}/zsh/.zprofile ~/.zprofile
	ln -sf ${DIR}/zsh/.zsh-history-substring-search.zsh \
	  ~/.zsh-history-substring-search.zsh

vim:
	mkdir -p ~/.vim/colors
	ln -sf ${DIR}/vim/.vimrc ~/.vimrc
	ln -sf ${DIR}/vim/acme-hbz.vim ~/.vim/colors/acme-hbz.vim
	curl -fLo ~/.vim/autoload/plug.vim --create-dirs \
	  https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim

nvim:
	mkdir -p ~/.config/nvim/colors
	ln -sf ${DIR}/vim/.vimrc ~/.config/nvim/init.vim
	ln -sf ${DIR}/vim/acme-hbz.vim ~/.config/nvim/colors/acme-hbz.vim
	curl -fLo ~/.local/share/nvim/site/autoload/plug.vim --create-dirs \
	  https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim

z:
	mkdir -p ~/lib
	curl -fLo ~/lib/z.sh https://raw.githubusercontent.com/rupa/z/master/z.sh

tmux:
	ln -sf ${DIR}/tmux/.tmux.conf ~/.tmux.conf

ghostty:
	mkdir -p ~/.config/ghostty
	ln -sf ${DIR}/ghostty/config ~/.config/ghostty/config

ai:
	mkdir -p ~/.claude/hooks
	ln -sf ${DIR}/agents/AGENTS.md ~/.claude/CLAUDE.md
	ln -sf ${DIR}/claude/settings.json ~/.claude/settings.json
	for f in ${DIR}/claude/hooks/*; do ln -sf "$$f" ~/.claude/hooks/; done
	curl -fLo ~/.claude/hooks/copy-claude-response \
	  https://raw.githubusercontent.com/Twizzes/copy-claude-response/main/copy-claude-response
	chmod +x ~/.claude/hooks/copy-claude-response

# --- Arch targets ---

dircolors:
	mkdir -p ~/.config/dircolors
	ln -sf ${DIR}/dircolors/config ~/.config/dircolors/config

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

DIR=$(shell pwd)

.PHONY: install mac arch wsl common zsh vim nvim tmux ghostty ai worktrunk brew brew-check git vscode macos-defaults z dircolors sway konsole mako wallpapers session-leds agent-deck agent-deck-daemon firmware help

install: mac ## Default target: full macOS install

help: ## List the documented targets
	@grep -hE '^[a-zA-Z0-9_-]+:.*## ' $(MAKEFILE_LIST) | sort | awk 'BEGIN{FS=":.*## "}{printf "  %-14s %s\n", $$1, $$2}'

mac: brew common ghostty macos-defaults agent-deck ## Full macOS setup (brew + common + ghostty + defaults)

arch: pkg common z dircolors sway mako konsole wallpapers ## Full Arch/sway setup

wsl: common ## WSL setup (common + Windows Terminal settings)
	ln -sf ${DIR}/WindowsTerminal/settings.json \
	  /mnt/c/Users/$$(cmd.exe /c echo %USERNAME% 2>/dev/null | tr -d '\r')/AppData/Local/Packages/Microsoft.WindowsTerminal_8wekyb3d8bbwe/LocalState/settings.json

common: zsh vim nvim tmux ai worktrunk git vscode session-leds ## Cross-platform configs (shell, editor, tmux, ai, git)

brew: ## Install Homebrew bundle (+ work overlay if present)
	brew bundle --file=${DIR}/brew/Brewfile
	@test -f ${DIR}/brew/Brewfile.work && brew bundle --file=${DIR}/brew/Brewfile.work || true

brew-check: ## Diff installed packages against the Brewfile
	@echo "Installed but not in Brewfile:"
	@brew bundle cleanup --file=${DIR}/brew/Brewfile 2>/dev/null | grep -v "^Would\|^Run\|^$$" || echo "  (none)"
	@echo "\nIn Brewfile but not installed:"
	@brew bundle check --file=${DIR}/brew/Brewfile --verbose 2>/dev/null | grep "^→" | sed 's/→ /  /' || echo "  (none)"

pkg: ## Install Arch packages from pacman/pkglist.txt
	sudo pacman -Syu
	sudo pacman -S --needed - < pacman/pkglist.txt

zsh: ## Symlink zsh config + fetch history-substring-search
	ln -sf ${DIR}/zsh/.zshrc ~/.zshrc
	ln -sf ${DIR}/zsh/.zprofile ~/.zprofile
	curl -fLo ~/.zsh-history-substring-search.zsh \
	  https://raw.githubusercontent.com/zsh-users/zsh-history-substring-search/master/zsh-history-substring-search.zsh

vim: ## Symlink vim config + colorscheme, fetch vim-plug
	mkdir -p ~/.vim/colors
	ln -sf ${DIR}/vim/.vimrc ~/.vimrc
	ln -sf ${DIR}/vim/acme-hbz.vim ~/.vim/colors/acme-hbz.vim
	curl -fLo ~/.vim/autoload/plug.vim --create-dirs \
	  https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim

nvim: ## Symlink nvim config (shares .vimrc) + fetch vim-plug
	mkdir -p ~/.config/nvim/colors
	ln -sf ${DIR}/vim/.vimrc ~/.config/nvim/init.vim
	ln -sf ${DIR}/vim/acme-hbz.vim ~/.config/nvim/colors/acme-hbz.vim
	curl -fLo ~/.local/share/nvim/site/autoload/plug.vim --create-dirs \
	  https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim

z: ## Fetch the z.sh directory jumper
	mkdir -p ~/lib
	curl -fLo ~/lib/z.sh https://raw.githubusercontent.com/rupa/z/master/z.sh

git: ## Wire git/.gitconfig into the global include path
	git config --global include.path ${DIR}/git/.gitconfig

session-leds: ## Symlink the agent session status CLI into ~/.local/bin
	mkdir -p ~/.local/bin
	ln -sf ${DIR}/keyboard/session-leds/bin/agent-status ~/.local/bin/agent-status

agent-deck: ## Build + install the direct-HID session LED daemon (needs rust)
	cd ${DIR}/keyboard/session-leds/agent-deck && cargo build --release
	mkdir -p ~/.local/bin
	install -m 755 ${DIR}/keyboard/session-leds/agent-deck/target/release/agent-deck \
	  ~/.local/bin/agent-deck

agent-deck-daemon: agent-deck ## Install + start launchd agent for agent-deck (macOS)
	@# clear the retired agent-leds unit, on a machine that still carries it
	launchctl bootout gui/$$(id -u)/io.hbz.agent-leds 2>/dev/null || true
	rm -f ~/Library/LaunchAgents/io.hbz.agent-leds.plist ~/.local/bin/agent-leds
	sed "s|__HOME__|$$HOME|g" ${DIR}/keyboard/session-leds/launchd/io.hbz.agent-deck.plist \
	  > ~/Library/LaunchAgents/io.hbz.agent-deck.plist
	launchctl bootout gui/$$(id -u)/io.hbz.agent-deck 2>/dev/null || true
	@# bootout returns before the job is gone, so bootstrap has to wait it out
	@for i in 1 2 3 4 5 6 7 8 9 10; do \
	  launchctl bootstrap gui/$$(id -u) ~/Library/LaunchAgents/io.hbz.agent-deck.plist \
	    2>/dev/null && exit 0; \
	  sleep 0.5; \
	done; echo "could not bootstrap io.hbz.agent-deck" >&2; exit 1

QMK_FORK ?= ~/dev/zsa-qmk

firmware: ## Build Voyager firmware from keyboard/voyager/src (needs qmk CLI + ZSA fork at QMK_FORK)
	ln -sfn ${DIR}/keyboard/voyager/src $(QMK_FORK)/keyboards/zsa/voyager/keymaps/hbz
	cd $(QMK_FORK) && PATH="$$(brew --prefix)/opt/arm-none-eabi-gcc@8/bin:$$(brew --prefix)/opt/arm-none-eabi-binutils/bin:$$PATH" \
	  qmk compile -kb zsa/voyager -km hbz
	cp $(QMK_FORK)/zsa_voyager_hbz.bin ${DIR}/keyboard/voyager/firmware/

vscode: ## Symlink VS Code settings (macOS or Linux path)
ifeq ($(shell uname),Darwin)
	mkdir -p ~/Library/Application\ Support/Code/User
	ln -sf ${DIR}/vscode/settings.json ~/Library/Application\ Support/Code/User/settings.json
else
	mkdir -p ~/.config/Code/User
	ln -sf ${DIR}/vscode/settings.json ~/.config/Code/User/settings.json
endif

tmux: ## Symlink tmux config, fetch tpm + resurrect/continuum plugins
	ln -sf ${DIR}/tmux/.tmux.conf ~/.tmux.conf
	@# tpm clones the rest itself, but resurrect and continuum are the
	@# crash-safety substrate claude-manager reads, so fetch them here
	@# and let `prefix+I` handle anything added to the conf later.
	mkdir -p ~/.tmux/plugins
	for p in tpm tmux-resurrect tmux-continuum; do \
	  [ -d ~/.tmux/plugins/$$p ] \
	    || git clone -q --depth 1 https://github.com/tmux-plugins/$$p ~/.tmux/plugins/$$p; \
	done
	@# custom save-command strategy — see the script for why the
	@# bundled `ps` one mis-saves Claude panes
	ln -sf ${DIR}/tmux/resurrect/save_command_strategies/pane-tty.sh \
	  ~/.tmux/plugins/tmux-resurrect/save_command_strategies/pane-tty.sh

ghostty: ## Symlink Ghostty config + acme-hbz theme
	mkdir -p ~/.config/ghostty/themes
	ln -sf ${DIR}/ghostty/config ~/.config/ghostty/config
	ln -sf ${DIR}/ghostty/acme-hbz ~/.config/ghostty/themes/acme-hbz

ai: ## Symlink Claude config (instructions, settings, hooks, skills, agents)
	@# instructions
	mkdir -p ~/.claude/hooks ~/.claude/skills ~/.claude/agents
	ln -sf ${DIR}/agents/AGENTS.md ~/.claude/CLAUDE.md
	@# settings — live file is gitignored (holds machine/work-local fields); merge the committed
	@# baseline under any existing local fields (local wins), so shared config self-heals each run
	@test -f ${DIR}/claude/settings.json || echo '{}' > ${DIR}/claude/settings.json
	@jq -s '.[0] * .[1]' ${DIR}/claude/settings.json.example ${DIR}/claude/settings.json > ${DIR}/claude/settings.json.tmp && mv ${DIR}/claude/settings.json.tmp ${DIR}/claude/settings.json
	ln -sf ${DIR}/claude/settings.json ~/.claude/settings.json
	@# hooks
	for f in ${DIR}/claude/hooks/*; do ln -sf "$$f" ~/.claude/hooks/; done
	@# personal skills (each subdir under claude/skills/ becomes ~/.claude/skills/<name>)
	for d in ${DIR}/claude/skills/*/; do ln -sfn "$${d%/}" ~/.claude/skills/; done
	@# personal agents (each .md under claude/agents/ becomes ~/.claude/agents/<name>.md)
	for f in ${DIR}/claude/agents/*.md; do ln -sf "$$f" ~/.claude/agents/; done
	@# external skills + tools
	curl -fsSL https://raw.githubusercontent.com/raine/git-surgeon/main/scripts/install.sh | bash
	git-surgeon install-skill --claude

worktrunk: ## Symlink worktrunk config
	mkdir -p ~/.config/worktrunk
	ln -sf ${DIR}/worktrunk/config.toml ~/.config/worktrunk/config.toml

macos-defaults: ## Apply macOS accent/highlight colour defaults
	defaults write NSGlobalDomain AppleAccentColor -int -1
	defaults write NSGlobalDomain AppleHighlightColor -string "1.000000 0.937255 0.690196 Yellow"

# --- Arch targets ---

dircolors: ## Symlink dircolors config (Arch)
	mkdir -p ~/.config/dircolors
	ln -sf ${DIR}/dircolors/config ~/.config/dircolors/config

sway: ## Symlink sway config + start script (Arch)
	mkdir -p ~/.config/sway
	ln -sf ${DIR}/sway/config ~/.config/sway/config
	ln -sf ${DIR}/sway/start-sway ~/start-sway

mako: ## Symlink mako notification config (Arch)
	mkdir -p ~/.config/mako
	ln -sf ${DIR}/mako/config ~/.config/mako/config

konsole: ## Symlink Konsole config + acme-hbz colorscheme (Arch)
	mkdir -p ~/.local/share/konsole
	ln -sf ${DIR}/konsole/konsolerc ~/.config/konsolerc
	ln -sf ${DIR}/konsole/DarkPastels.colorscheme \
	  ~/.local/share/konsole/DarkPastels.colorscheme
	ln -sf ${DIR}/konsole/AcmeHbz.colorscheme \
	  ~/.local/share/konsole/AcmeHbz.colorscheme
	ln -sf ${DIR}/konsole/hbz.profile ~/.local/share/konsole/hbz.profile

wallpapers: ## Symlink wallpapers (Arch)
	mkdir -p ~/.config/wallpapers
	ln -sf ${DIR}/wallpapers/darkgrey.png ~/.config/wallpapers/darkgrey.png
	ln -sf ${DIR}/wallpapers/white.png ~/.config/wallpapers/white.png

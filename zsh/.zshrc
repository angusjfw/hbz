# pure prompt
autoload -U promptinit && promptinit
prompt pure

# better completion
autoload -U compinit && compinit
zstyle ':completion:*' menu select matcher-list 'm:{a-zA-Z}={A-Za-z}'

# shared history between all zsh instances
HISTFILE=~/.zsh_history
HISTSIZE=SAVEHIST=100000
setopt sharehistory
setopt extendedhistory
setopt HIST_IGNORE_SPACE
setopt HIST_IGNORE_ALL_DUPS

# general
export EDITOR=nvim
export KEYTIMEOUT=1
export LC_ALL=en_GB.UTF-8
export LANG=en_GB.UTF-8

# zsh history (load before highlighting)
source "$HOME/.zsh-history-substring-search.zsh"
zmodload zsh/terminfo
bindkey '^[[A' history-substring-search-up
bindkey '^[[B' history-substring-search-down
bindkey -M vicmd 'k' history-substring-search-up
bindkey -M vicmd 'j' history-substring-search-down

# styling
source /usr/share/zsh/plugins/zsh-syntax-highlighting/zsh-syntax-highlighting.zsh

# base16
BASE16_SHELL=$HOME/.config/base16-shell/
[ -n "$PS1" ] && [ -s $BASE16_SHELL/profile_helper.sh ] && eval "$($BASE16_SHELL/profile_helper.sh)"

# tmux
PS=1"$PS1"'$([ -n "$TMUX" ] && tmux setenv TMUXPWD_$(tmux display -p "#D" | tr -d %) "$PWD")'

# fzf
[ -f ~/.fzf.zsh ] && source ~/.fzf.zsh
export FZF_DEFAULT_COMMAND='ag -g ""'
export FZF_DEFAULT_OPTS='--color fg+:5,hl+:6'

# ssh keys
# eval $(keychain --eval --quiet --nogui --noask id_rsa google_compute_engine)
export SSH_AUTH_SOCK="$XDG_RUNTIME_DIR/ssh-agent.socket"
export DKR_COMPOSE_PREFIX="SSH_AUTH_SOCK=${SSH_AUTH_SOCK}"

# lscolors
eval $(dircolors $HOME/.config/dircolors/config)

source /usr/lib/z.sh
source ~/.zprofile
if [ -f ~/.zworkprofile ]; then
  source ~/.zworkprofile
fi

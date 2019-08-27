typeset -U path
path=(~/bin $path[@])

alias slay='~/start-sway'
alias stway='~/start-sway us external'
alias shway='~/start-sway us external hi'
alias ksway='pkill -15 sway'
alias lock='swaylock -c 444444'

alias pacin='sudo pacman -S'
alias pacup='sudo pacman -Syu && yaourt -Syua'

alias sudo='sudo '
alias vim='nvim'
alias pls='sudo $(fc -ln -1)'
alias clr='clear'
alias ls='ls -a --color'
alias lls='ls -Alhtr'
alias lss='ls -lhta'
alias findls='find . -type l -exec ls -l {} \; | grep'
alias bonsai="tree -I 'tmp|node_modules|bower_components'"
alias xclip='xclip -selection clipboard'
alias say='echo "$1" | espeak -s 120 2>/dev/null'
alias pingg='ping 8.8.8.8'

alias gs='git status '
alias ga='git add '
alias gb='git branch '
alias gco='git commit'
alias gd='git diff'
alias gch='git checkout '
alias glog="git log --graph --abbrev-commit --decorate --date=relative --format=format:'%C(bold blue)%h%C(reset) - %C(bold green)(%ar)%C(reset) %C(white)%s%C(reset) %C(dim white)- %an%C(reset)%C(bold yellow)%d%C(reset)' --all"
alias gk='gitk --all&'
alias screenie='grim -g "$(slurp)" $(xdg-user-dir PICTURES)/$(date +"%Y-%m-%d-%H%M%S_screenshot.png")'

chkey() {
  export XKB_DEFAULT_LAYOUT=$1
  setxkbmap $1
}

dison() {
  swaymsg output $1 enable
}

disoff() {
  swaymsg output $1 disable
}

dissc() {
  swaymsg output $1 scale $2
}


tmp() {
  nvim $(mktemp /tmp/$1-XXXXXX.$2)
}

v() {
  if [[ $# -eq 0 ]]; then
    command nvim .;
  else
    command nvim "$@";
  fi
}

git() {
    if [[ $1 == "reset" && $2 == "--hard" ]]; then
        while true; do
          read '?Are you sure?' answer
          case $answer in
            hard* ) command git "$@"; break;;
            [Nn]* ) break;;
            * ) echo -e "Type 'hard' to confirm or n.";;
          esac
        done
    else
        command git "$@"
    fi
}

post-json() {
  url=$1; data=$2
  if [[ -z $url || -z $data ]]; then
    echo 'usage: post-json <url> <json file or string>'
    exit 1
  fi

  if [[ ${data: -5} == ".json" ]]; then
    curl $url -X POST -H 'Content-Type: application/json' -d @$data
  else
    curl $url -X POST -H 'Content-Type: application/json' -d $data
  fi
}

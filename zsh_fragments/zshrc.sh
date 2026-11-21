# shellcheck shell=bash

# aliases

alias ls='"$HOMEBREW_PREFIX/opt/coreutils/libexec/gnubin/ls" --color=auto --show-control-char'
alias show-my-ip='dig +short -t txt o-o.myaddr.l.google.com @8.8.8.8'
alias fnox-add-secret='~/.local/bin/dotfiles-local fnox-add-secret'
alias to-snake-case='pbpaste | sed -E "s/([a-z0-9])([A-Z])/\1_\2/g; s/([A-Z]+)([A-Z][a-z])/\1_\2/g; s/[^[:alnum:]]+/_/g; s/^_+//; s/_+$//" | tr "[:upper:]" "[:lower:]" | pbcopy'
alias git-branch='git branch --sort=committerdate'
alias git-sweep-branches='~/.local/bin/dotfiles-local git-sweep-branches'
alias git-sync-base='~/.local/bin/dotfiles-local git-sync-base'

# functions

function with-new-terminal() {
  ~/.local/bin/dotfiles-local with-new-terminal "$@"
}

function _echo_gnu_paths() {
  local gnu_paths=(
    "$HOMEBREW_PREFIX/opt/coreutils/libexec/gnubin"
    "$HOMEBREW_PREFIX/opt/findutils/libexec/gnubin"
    "$HOMEBREW_PREFIX/opt/gnu-sed/libexec/gnubin"
    "$HOMEBREW_PREFIX/opt/gnu-tar/libexec/gnubin"
    "$HOMEBREW_PREFIX/opt/gawk/libexec/gnubin"
    "$HOMEBREW_PREFIX/opt/grep/libexec/gnubin"
  )
  (
    IFS=:
    echo "${gnu_paths[*]}"
  )
}

function gnu-activate() {
  local gnu_paths
  gnu_paths="$(_echo_gnu_paths)"
  export PATH="$gnu_paths:$PATH"
}

function gnu-deactivate() {
  local gnu_paths
  gnu_paths="$(_echo_gnu_paths)"
  export PATH=${PATH//$gnu_paths:/}
}

function show-modified-homebrew-formula() {
  (
    cd "$(brew --repository "${1:-homebrew/core}")" &&
      git status --short
  )
}

# settings

export PROMPT="%K{2} %k %F{7}[%DT%* %n@%m:%~]%f"$'\n'"\$ "

JAVA_HOME=$(/usr/libexec/java_home)
export JAVA_HOME=$JAVA_HOME

gnu-activate

eval "$(dircolors -b "$HOME"/.dircolors)"
eval "$(mise activate zsh)"

# Deactivate flow control to allow Ctrl-S for forward history search
[[ $- == *i* ]] && stty stop undef

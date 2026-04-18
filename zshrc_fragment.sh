# shellcheck shell=bash

DOTFILES_DIR="${0:A:h}"

# aliases

alias ls='"$HOMEBREW_PREFIX/opt/coreutils/libexec/gnubin/ls" --color=auto --show-control-char'
alias show-my-ip='dig +short -t txt o-o.myaddr.l.google.com @8.8.8.8'
alias with-new-terminal='elixir "$DOTFILES_DIR/scripts/with_new_terminal.exs"'

# functions

function _echo_gnu_paths() {
  gnu_paths="$HOMEBREW_PREFIX/opt/coreutils/libexec/gnubin"
  gnu_paths="$HOMEBREW_PREFIX/opt/findutils/libexec/gnubin:$gnu_paths"
  gnu_paths="$HOMEBREW_PREFIX/opt/gnu-sed/libexec/gnubin:$gnu_paths"
  gnu_paths="$HOMEBREW_PREFIX/opt/gnu-tar/libexec/gnubin:$gnu_paths"
  gnu_paths="$HOMEBREW_PREFIX/opt/gawk/libexec/gnubin:$gnu_paths"
  gnu_paths="$HOMEBREW_PREFIX/opt/grep/libexec/gnubin:$gnu_paths"
  echo "$gnu_paths"
}

function gnu-activate() {
  gnu_paths="$(_echo_gnu_paths)"
  export PATH="$gnu_paths:$PATH"
}

function gnu-deactivate() {
  gnu_paths="$(_echo_gnu_paths)"
  export PATH=${PATH//$gnu_paths/}
}

function show-modified-homebrew-formula() {
  (
    cd "$(brew --repository "${1:-homebrew/core}")" &&
      git status --short
  )
}

# settings

export LC_CTYPE=ja_JP.UTF-8
export PROMPT="%K{2} %k %F{7}[%DT%* %n@%m:%~]%f"$'\n'"\$ "

JAVA_HOME=$(/usr/libexec/java_home)
export JAVA_HOME=$JAVA_HOME

gnu-activate

eval "$(dircolors -b "$HOME"/.dircolors)"
eval "$(mise activate zsh)"

# Deactivate flow control to allow Ctrl-S for forward history search
[[ $- == *i* ]] && stty stop undef

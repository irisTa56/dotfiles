# shellcheck shell=bash
# Sourced from ~/.zprofile, which login shells read.

# Login-shell file, because /etc/zprofile runs path_helper first
# and it would demote Homebrew: https://github.com/orgs/Homebrew/discussions/1127
eval "$(/opt/homebrew/bin/brew shellenv)"

# uv installs its managed Pythons here, and `uv tool install` lands here too.
# Not .zshenv, whose PATH /etc/zprofile's path_helper demotes in a login shell;
# after brew shellenv, which prepends /opt/homebrew/bin ahead of whatever
# PATH holds, so above it Homebrew would answer for anything both provide.
export PATH="$HOME/.local/bin:$PATH"

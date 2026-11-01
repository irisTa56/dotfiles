# shellcheck shell=bash
# Sourced from ~/.zprofile, which login shells read.

# Shims reach mise's tools from a shell that never runs `mise activate` in .zshrc.
# Above brew shellenv, so that Homebrew answers for anything both provide:
# a repository's mise.toml can install a tool Homebrew also has, and a shim
# in front would then fail wherever a mise.toml cannot be parsed.
eval "$(/opt/homebrew/bin/mise activate zsh --shims)"

# Login-shell file, because /etc/zprofile runs path_helper first
# and it would demote Homebrew: https://github.com/orgs/Homebrew/discussions/1127
eval "$(/opt/homebrew/bin/brew shellenv)"

# uv installs its managed Pythons here, and `uv tool install` lands here too.
# Not .zshenv, whose PATH /etc/zprofile's path_helper demotes in a login shell;
# after brew shellenv, which prepends /opt/homebrew/bin
# ahead of whatever PATH holds,
# so above it Homebrew would answer for anything both provide.
export PATH="$HOME/.local/bin:$PATH"

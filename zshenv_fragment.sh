# shellcheck shell=bash
# Sourced from ~/.zshenv, which every zsh reads,
# a tool's non-interactive one included,
# so what an agent's shell needs goes here.

# A tool's shell reads neither .zprofile, where brew shellenv sets this,
# nor .zshrc — but Claude Code's shell snapshot restores the ls alias
# that expands this, and the alias exits 127 when it is unset.
export HOMEBREW_PREFIX="${HOMEBREW_PREFIX:-/opt/homebrew}"

# A tool's shell reads this file and no other startup file:
# non-interactive rules out .zshrc, and .zprofile has not run —
# its login option is replayed by the harness snapshot, not earned.
# It runs commands written for bash, where an unmatched glob is inert.
[[ -o interactive ]] || setopt nonomatch

# Here rather than .zshrc,
# so a tool's shell handles characters the same way an interactive one does;
# Claude Code's snapshot does not carry it over.
export LC_CTYPE=ja_JP.UTF-8

# rclone reads its config password from the login keychain,
# so the config can stay encrypted without a prompt;
# the README's shell startup section has the setup.
# Here rather than mise's [env],
# which only `mise activate` in an interactive shell applies.
export RCLONE_PASSWORD_COMMAND="/usr/bin/security find-generic-password -a rclone -s config -w"

# uvx and `uv tool` pick no package version published less than a day ago,
# the window npm gets from its user config (set by setup_dotfiles.sh).
# uv gets it only where no project lock is written:
# a user-wide one is baked into each project's uv.lock
# and then conflicts with anyone locking without it (astral-sh/uv#18775),
# so a project sets its own in pyproject.toml.
# The window goes as a flag rather than UV_EXCLUDE_NEWER,
# which the tool uvx launches would inherit
# and pass to a uv it runs in the project.
# Setting UV_EXCLUDE_NEWER (false to disable) replaces the flag.
uvx() {
  if [[ -v UV_EXCLUDE_NEWER ]]; then
    command uvx "$@"
  else
    command uvx --exclude-newer "1 day" "$@"
  fi
}
uv() {
  if [[ ${1-} == tool && ! -v UV_EXCLUDE_NEWER ]]; then
    case ${2-} in
    install | run | upgrade)
      command uv tool "$2" --exclude-newer "1 day" "${@:3}"
      return
      ;;
    esac
  fi
  command uv "$@"
}

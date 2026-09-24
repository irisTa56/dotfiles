#!/bin/bash
set -euxo pipefail

curl -s -o ~/.dircolors https://raw.githubusercontent.com/trapd00r/LS_COLORS/master/LS_COLORS

mkdir -p ~/.config/git
cat <<'EOF' >~/.config/git/ignore
__pycache__/
__tmp*
_tmp*/
.DS_Store
.env.local
.git/
.ipynb_checkpoints/
.pytest_cache/
.tmp.drive*
**/.claude/settings.local.json
*uncommitted*/
CLAUDE.local.md
erl_crash.dump
mise.local.toml
EOF

cat <<'EOF' >~/.zprofile
# Login-shell file, because /etc/zprofile runs path_helper first
# and it would demote Homebrew: https://github.com/orgs/Homebrew/discussions/1127
eval $(/opt/homebrew/bin/brew shellenv)

# uv installs its managed Pythons here, and `uv tool install` lands here too.
# Not .zshenv, whose PATH /etc/zprofile's path_helper demotes in a login shell;
# after brew shellenv, which prepends /opt/homebrew/bin ahead of whatever
# PATH holds, so above it Homebrew would answer for anything both provide.
export PATH="$HOME/.local/bin:$PATH"
EOF

cat <<'EOF' >~/.zshenv
# A tool's shell reads neither .zprofile, where brew shellenv sets this,
# nor .zshrc — but Claude Code's shell snapshot restores the ls alias
# that expands this, and the alias exits 127 when it is unset.
export HOMEBREW_PREFIX="${HOMEBREW_PREFIX:-/opt/homebrew}"

# A tool's shell reads this file and no other startup file: non-interactive
# rules out .zshrc, and .zprofile has not run — its login option is replayed
# by the harness snapshot, not earned. It runs commands written for bash,
# where an unmatched glob is inert.
[[ -o interactive ]] || setopt nonomatch

# rclone reads its config password from the login keychain, so the config
# can stay encrypted without a prompt; the README's shell startup section has
# the setup. Here rather than mise's [env], which only an interactive shell's
# `mise activate` applies.
export RCLONE_PASSWORD_COMMAND="/usr/bin/security find-generic-password -a rclone -s config -w"

# uvx and `uv tool` pick no package version published less than a day ago,
# the window npm gets from its user config (set below). uv gets it only
# where no project lock is written: a user-wide one is baked into each
# project's uv.lock and then conflicts with anyone locking without it
# (astral-sh/uv#18775), so a project sets its own in pyproject.toml.
# The window goes as a flag rather than UV_EXCLUDE_NEWER, which the tool
# uvx launches would inherit and pass to a uv it runs in the project.
# Setting UV_EXCLUDE_NEWER (false to disable) replaces the flag.
uvx() {
  if [[ -v UV_EXCLUDE_NEWER ]]; then
    command uvx "$@"
  else
    command uvx --exclude-newer "1 day" "$@"
  fi
}
uv() {
  if [[ ${1-} == tool && ${2-} == (install|run|upgrade) && ! -v UV_EXCLUDE_NEWER ]]; then
    command uv tool "$2" --exclude-newer "1 day" "${@:3}"
  else
    command uv "$@"
  fi
}
EOF

# npm picks no package version published less than a day ago,
# so a compromised release pulled within hours is not taken into a lockfile
# or a one-off install; mise and pnpm 11 already wait a day by default.
# A version a lockfile already pins still installs. User config rather than
# an environment variable, which would outrank a project's own .npmrc.
# One key set in place: the file may also hold registry auth.
npm config set min-release-age=1 --location=user

# launchd starts the pitchfork supervisor with no shell environment,
# and its default `sh -c` reads no startup file, so a daemon running rclone
# would get no RCLONE_PASSWORD_COMMAND and stall on the password prompt.
# zsh reads .zshenv even non-interactively. One key set in place: the file
# also holds the namespaces `mise daemons` registers, which differ per machine.
pitchfork settings set --global general.shell "/bin/zsh -c"

#!/bin/bash
set -euxo pipefail

curl -s -o ~/.dircolors https://raw.githubusercontent.com/trapd00r/LS_COLORS/master/LS_COLORS

mkdir -p ~/.config/git
cat <<'EOF' >~/.config/git/ignore
__pycache__/
__tmp*
_tmp*/
.DS_Store
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

# uv installs its managed Pythons here, and uv tool install lands here too.
# Not .zshenv, whose PATH /etc/zprofile's path_helper demotes;
# after brew shellenv, which prepends /opt/homebrew/bin ahead of whatever
# PATH holds, so above it Homebrew would answer for anything both provide.
export PATH="$HOME/.local/bin:$PATH"
EOF

cat <<'EOF' >~/.zshenv
# A tool's shell reads neither .zprofile, where brew shellenv sets this,
# nor .zshrc — but Claude Code's shell snapshot restores the ls alias
# that expands this, and the alias exits 127 when it is unset.
export HOMEBREW_PREFIX="${HOMEBREW_PREFIX:-/opt/homebrew}"

# Not .zprofile (login-only) or .zshrc (interactive-only):
# a tool's shell is neither, and it runs commands written for bash,
# where an unmatched glob is inert.
[[ -o interactive ]] || setopt nonomatch
EOF

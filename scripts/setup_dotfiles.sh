#!/bin/bash
set -euxo pipefail

curl -s -o ~/.dircolors https://raw.githubusercontent.com/trapd00r/LS_COLORS/master/LS_COLORS

mkdir -p ~/.config/git
cat <<'EOF' >~/.config/git/ignore
__pycache__/
__tmp*
_tmp*/
.DS_Store
.ipynb_checkpoints/
.pytest_cache/
.tmp.drive*
**/.claude/settings.local.json
*uncommitted*/
CLAUDE.local.md
erl_crash.dump
EOF

cat <<'EOF' >~/.zprofile
eval $(/opt/homebrew/bin/brew shellenv)
EOF

cat <<'EOF' >~/.zshenv
# uv
export PATH="$HOME/.local/bin:$PATH"

# Export HOMEBREW_PREFIX (the variable only, NOT `brew shellenv`) so non-login
# tool shells, which skip .zprofile, can still resolve $HOMEBREW_PREFIX-expanding
# aliases like the `ls` alias in zshrc_fragment.sh. PATH precedence stays in
# .zprofile; see the dotfiles README ("Shell startup: .zprofile vs .zshenv").
export HOMEBREW_PREFIX="${HOMEBREW_PREFIX:-/opt/homebrew}"
EOF

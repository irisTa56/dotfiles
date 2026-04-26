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
erl_crash.dump
EOF

cat <<'EOF' >~/.zprofile
eval $(/opt/homebrew/bin/brew shellenv)
EOF

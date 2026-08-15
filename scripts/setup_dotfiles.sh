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
# uv installs its managed Pythons here; in a login shell a prepend in .zshenv runs before path_helper and gets demoted.
export PATH="$HOME/.local/bin:$PATH"

# After the line above, so Homebrew keeps precedence over uv's Pythons.
eval $(/opt/homebrew/bin/brew shellenv)
EOF

cat <<'EOF' >~/.zshenv
# Non-login shells skip .zprofile, where brew shellenv sets this.
export HOMEBREW_PREFIX="${HOMEBREW_PREFIX:-/opt/homebrew}"

# Non-interactive shells here run commands written for bash, where an unmatched glob is inert.
[[ -o interactive ]] || setopt nonomatch
EOF

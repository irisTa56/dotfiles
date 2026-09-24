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
fnox.local.toml
mise.local.toml
EOF

# Each startup file sources its fragment from the main checkout,
# so an edit there reaches the next shell without rerunning this script,
# and lines an installer appends to the file stay.
# Not a worktree's path, which goes away.
# A missing fragment (the checkout moved, or sits on a branch without it)
# is reported on every shell start rather than skipped,
# since unattended shells depend on .zshenv's.
# A file that already names its fragment, however it sources it, is left alone.
# The line starts with a newline in case the file's last line lacks one,
# which would otherwise glue the two.
dotfiles_dir="$(git -C "$(dirname "$0")" worktree list --porcelain | sed -n '1s/^worktree //p')"
for name in zshenv zprofile zshrc; do
  fragment="$dotfiles_dir/zsh_fragments/$name.sh"
  touch ~/."$name"
  grep -qF "zsh_fragments/$name.sh" ~/."$name" ||
    printf '\nif [[ -r "%s" ]]; then source "%s"; else echo "%s: missing, not sourced" >&2; fi\n' \
      "$fragment" "$fragment" "$fragment" >>~/."$name"
done

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

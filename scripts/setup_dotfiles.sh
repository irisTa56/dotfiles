#!/bin/bash
set -euxo pipefail

# Replaced only by a download that succeeded, so a failed one on a rerun
# stops here rather than leaving an error page where the colors were.
curl -fsSL -o ~/.dircolors.new https://raw.githubusercontent.com/trapd00r/LS_COLORS/master/LS_COLORS
mv ~/.dircolors.new ~/.dircolors

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

# rclone's config is encrypted with a password that RCLONE_PASSWORD_COMMAND
# reads from the login keychain. The command is taken from zsh, whose .zshenv
# states it, so it is written once; this run's own environment may predate
# the sourcing line above. Storing the password elsewhere, fnox say,
# means changing that command and the item created below together.
RCLONE_PASSWORD_COMMAND="$(zsh -c 'printf %s "$RCLONE_PASSWORD_COMMAND"')"
test -n "$RCLONE_PASSWORD_COMMAND"
export RCLONE_PASSWORD_COMMAND
# A random password nobody types, so the item can be made here;
# xtrace is off so the value is not echoed.
if ! $RCLONE_PASSWORD_COMMAND >/dev/null 2>&1; then
  { set +x; } 2>/dev/null
  security add-generic-password -a rclone -s config -w "$(openssl rand -base64 40)"
  set -x
fi
# `check` fails both on a config not yet encrypted, a missing one included,
# and on one encrypted with another password, which `set` cannot open;
# the header tells the two apart.
if ! rclone config encryption check >/dev/null 2>&1; then
  rclone_conf="$(rclone config paths | sed -n 's/^Config file: *//p')"
  if grep -qx 'RCLONE_ENCRYPT_V0:' "$rclone_conf" 2>/dev/null; then
    echo "$rclone_conf is encrypted with another password; decrypt it with \`env -u RCLONE_PASSWORD_COMMAND rclone config encryption remove\`, which asks for that password, or without it move the file aside, then rerun" >&2
    exit 1
  fi
  rclone config encryption set
fi

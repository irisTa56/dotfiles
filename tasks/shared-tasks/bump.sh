#!/usr/bin/env bash
#MISE description="Move this repository's pin of dotfiles' shared tasks to dotfiles' main, and list what the move changes under tasks/"
#
# Whether a move is due is shared-tasks:check's answer, so a main that moved without
# touching tasks/ asks for none. Running each changed task, and deciding whether a new one
# joins this repository's hooks or CI, is left to whoever moves the pin.
set -euo pipefail

remote_url="https://github.com/irisTa56/dotfiles.git"
include="git::${remote_url}//tasks?ref="
config=mise.toml

# 0: nothing to move, dotfiles itself included; 2: it could not tell. Either way stop there.
status=0
mise run shared-tasks:check || status=$?
if [ "$status" -ne 1 ]; then
  exit "$status"
fi

if [ "$(grep -c "${include}[0-9a-f]\{40\}" "$config")" -ne 1 ]; then
  echo "[fail] expected exactly one pin of ${include} in $config" >&2
  exit 2
fi
old=$(grep -o "${include}[0-9a-f]\{40\}" "$config" | sed 's/.*ref=//')
new=$(git ls-remote "$remote_url" refs/heads/main | cut -f1)

# Trees without blobs, into a throwaway repository, as shared-tasks:check fetches them.
scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
git -C "$scratch" init -q
git -C "$scratch" fetch -q --depth=1 --filter=blob:none "$remote_url" "$old" "$new"
git -C "$scratch" diff --name-status --no-renames "$old" "$new" -- tasks

# Not sed -i, whose form differs between GNU and BSD sed; writing through the file keeps its mode.
sed "s|${include}${old}|${include}${new}|" "$config" >"$scratch/config"
cat "$scratch/config" >"$config"

mise run shared-tasks:check
echo "Run each changed task listed above before committing the move."

#!/usr/bin/env bash
# Merge the tracked .claude/settings.base.json into the machine's ~/.claude/settings.json.
# The machine file keeps whatever else it holds (hooks `rtk init` patched in, keys the
# /config panel wrote), which is why the base is merged rather than symlinked.
# Objects merge key by key, arrays gain the base's entries they lack, and any other
# base value replaces the machine's, so a rerun changes nothing.
# Removing an entry from the base does not remove it here; edit the machine file for that.
set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
base="$script_dir/../.claude/settings.base.json"
target="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/settings.json"

current='{}'
[ -f "$target" ] && current=$(cat "$target")

merged=$(jq --argjson base "$(cat "$base")" '
  def merge($b):
    if type == "object" and ($b | type) == "object" then
      reduce ($b | keys_unsorted[]) as $k (.; .[$k] |= merge($b[$k]))
    elif type == "array" and ($b | type) == "array" then
      . + ($b - .)
    else
      $b
    end;
  merge($base)
' <<<"$current")

if [ "$merged" = "$(jq . <<<"$current")" ]; then
  echo "$target already holds everything in the base"
  exit 0
fi

diff -u <(jq . <<<"$current") <(echo "$merged") || true
tmp=$(mktemp "$target.XXXXXX")
echo "$merged" >"$tmp"
mv "$tmp" "$target"
echo "updated $target"

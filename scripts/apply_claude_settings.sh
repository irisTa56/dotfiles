#!/usr/bin/env bash
# Merge the tracked .claude/settings.base.json into the machine's ~/.claude/settings.json.
# The machine file keeps whatever else it holds (hooks `rtk init` patched in, keys the
# /config panel wrote), which is why the base is merged rather than symlinked.
# Objects merge key by key; an array keeps the machine's own entries and ends with the
# base's in the base's order, which a `!` carve-out needs to follow the rule it carves
# from even after the machine file was sorted; any other base value replaces the
# machine's. A rerun changes nothing.
# Removing an entry from the base does not remove it here; edit the machine file for that.
set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
base="$script_dir/../.claude/settings.base.json"
target="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/settings.json"

# An empty file reads as {} so it is filled rather than reported as already merged.
current='{}'
[ -f "$target" ] && current=$(jq -s 'add // {}' "$target")

merged=$(jq --argjson base "$(cat "$base")" '
  def merge($b):
    if type == "object" and ($b | type) == "object" then
      reduce ($b | keys_unsorted[]) as $k (.; .[$k] |= merge($b[$k]))
    elif type == "array" and ($b | type) == "array" then
      (. - $b) + $b
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

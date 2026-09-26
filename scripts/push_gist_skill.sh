#!/usr/bin/env bash
# Push local edits to a gist-sourced skill back to its gist, then move the pin to it.
# APM deploys the skill from apm.yml's gist dependency (aliased to the skill's name) and
# restores the pinned content on every `apm install`, so a local edit is lost unless it is
# pushed here and the lockfile advanced past it.
# Usage: push_gist_skill.sh <name>
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
lockfile="$root/apm.lock.yaml"

name="${1:-}"
# Guard the name: it becomes a path component.
case "$name" in
"" | . | .. | */*)
  echo "[fail] ${name:-<empty>}: invalid skill name" >&2
  exit 1
  ;;
esac

# A field of the skill's gist dependency in the lockfile.
locked() {
  N="$name" yq -r ".dependencies[] | select(.name == strenv(N) and .host == \"gist.github.com\") | .$1" "$lockfile"
}
head_of_gist() { gh api "/gists/$gist_id" --jq '.history[0].version'; }

# repo_url is <owner>/<gist_id> for a gist dependency.
repo=$(locked repo_url)
gist_id="${repo##*/}"
if ! printf '%s' "$gist_id" | grep -Eq '^[0-9a-f]+$'; then
  echo "[fail] $name: no gist dependency by that alias in $(basename "$lockfile")" >&2
  exit 1
fi

file="$root/.claude/skills/$name/SKILL.md"
if [ ! -f "$file" ]; then
  echo "[fail] $name: no local SKILL.md at ${file#"$root"/} (run 'apm install' first)" >&2
  exit 1
fi

# The local copy is the pinned commit's, so pushing it over a newer gist head would drop
# whatever that head added.
pinned=$(locked resolved_commit)
latest=$(head_of_gist)
if [ "$pinned" != "$latest" ]; then
  echo "[fail] $name: the local copy is from $pinned but gist $gist_id is at $latest; set your edit aside, run 'apm update --yes $name', and redo it on top" >&2
  exit 1
fi

# PATCH via the API. `gh gist edit -f ... < stdin` silently no-ops in a non-interactive
# shell (exits 0, leaves the gist unchanged), so it must not be used here.
jq -n --rawfile c "$file" '{files: {"SKILL.md": {content: $c}}}' |
  gh api -X PATCH "/gists/$gist_id" --input - >/dev/null

# Verify: both command substitutions strip trailing newlines, so GitHub's added trailing
# newline does not show as a spurious mismatch. The /raw/ URL is CDN-cached, so read the
# API, not the raw URL.
local_content=$(cat "$file")
remote_content=$(gh api "/gists/$gist_id" --jq '.files["SKILL.md"].content')
if [ "$local_content" != "$remote_content" ]; then
  echo "[fail] $name: pushed but gist $gist_id still differs from local; inspect manually" >&2
  exit 1
fi
echo "[ok] $name -> gist $gist_id"

# Pin the pushed commit, so the next `apm install` restores the edit rather than reverting it.
cd "$root"
apm update --yes "$name"

# The pin comes from the gist's git side, which the API write may not have reached yet;
# a stale pin would restore the old copy on the next install while this reported success.
pinned=$(locked resolved_commit)
latest=$(head_of_gist)
if [ "$pinned" != "$latest" ]; then
  echo "[fail] $name: pinned $pinned but gist $gist_id is at $latest; the edit is in the gist, the local copy is the old one, so rerun 'apm update --yes $name' until apm.lock.yaml pins $latest" >&2
  exit 1
fi
echo "[note] commit apm.lock.yaml and land it on main, or an install from main restores the old copy"

#!/usr/bin/env bash
# Push local edits to a gist-sourced skill back to its gist.
# The materialized .claude/skills/<name>/SKILL.md is gitignored and Gist->local only
# (see sync_gist_skills.sh); the gist is the only durable copy, so a local edit is lost
# on the next `skills:sync` unless pushed back here.
# Catalog: gistSkills.json (SoT) maps skill name -> raw gist URL; the gist id and filename
# are parsed from that URL.
# Usage: push_gist_skill.sh <name>
set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
root="$script_dir/.."
catalog="$root/gistSkills.json"
skills_dir="$root/.claude/skills"

[ -f "$catalog" ] || {
  echo "catalog not found: $catalog" >&2
  exit 1
}

name="${1:-}"
# Guard the catalog key: it becomes a path component, same as in sync_gist_skills.sh.
case "$name" in
"" | . | .. | */*)
  echo "[fail] ${name:-<empty>}: invalid skill name" >&2
  exit 1
  ;;
esac

url=$(jq -r --arg n "$name" '.[$n] // empty' "$catalog")
if [ -z "$url" ]; then
  echo "[fail] $name: not in $(basename "$catalog")" >&2
  exit 1
fi

file="$skills_dir/$name/SKILL.md"
if [ ! -f "$file" ]; then
  echo "[fail] $name: no local SKILL.md at ${file#"$root"/} (run 'skills:sync $name' first)" >&2
  exit 1
fi

# Parse gist id and filename out of the raw URL, e.g.
#   https://gist.githubusercontent.com/<owner>/<gist_id>/raw/[<sha>/]<filename>
# Anchor on the exact host so a non-gist URL fails loudly here instead of silently
# yielding a bogus gist id that the hex check below might still pass.
case "$url" in
https://gist.githubusercontent.com/*) ;;
*)
  echo "[fail] $name: not a gist URL in $(basename "$catalog"): $url" >&2
  exit 1
  ;;
esac
path="${url#https://gist.githubusercontent.com/}"
before_raw="${path%%/raw/*}"
gist_id="${before_raw##*/}"
after_raw="${path#*/raw/}"
filename="${after_raw##*/}"
if ! printf '%s' "$gist_id" | grep -Eq '^[0-9a-f]+$' || [ -z "$filename" ]; then
  echo "[fail] $name: could not parse gist id/filename from $url" >&2
  exit 1
fi

# PATCH via the API. `gh gist edit -f ... < stdin` silently no-ops in a non-interactive
# shell (exits 0, leaves the gist unchanged), so it must not be used here.
jq -n --arg fn "$filename" --rawfile c "$file" '{files: {($fn): {content: $c}}}' |
  gh api -X PATCH "/gists/$gist_id" --input - >/dev/null

# Verify: both command substitutions strip trailing newlines, so GitHub's added trailing
# newline does not show as a spurious mismatch. The /raw/ URL is CDN-cached, so read the
# API, not the raw URL.
local_content=$(cat "$file")
remote_content=$(FN="$filename" gh api "/gists/$gist_id" --jq '.files[env.FN].content')
if [ "$local_content" = "$remote_content" ]; then
  echo "[ok] $name -> gist $gist_id ($filename)"
else
  echo "[fail] $name: pushed but gist $gist_id still differs from local; inspect manually" >&2
  exit 1
fi

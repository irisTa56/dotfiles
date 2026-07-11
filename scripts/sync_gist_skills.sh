#!/usr/bin/env bash
# Vendor gist-sourced single-file skills into .claude/skills/<name>/SKILL.md.
# Catalog: gistSkills.json (SoT) maps skill name -> raw gist URL.
# Materialized SKILL.md files are gitignored like APM deps; the catalog+script is the source of truth.
# Usage: sync_gist_skills.sh [name]   # no name = sync every catalog entry
set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
root="$script_dir/.."
catalog="$root/gistSkills.json"
skills_dir="$root/.claude/skills"

[ -f "$catalog" ] || {
  echo "catalog not found: $catalog" >&2
  exit 1
}

# Fetch one skill atomically: temp lives in the destination dir so mv is a same-fs rename(2).
sync_one() {
  local name="$1" url dir tmp
  # Guard the catalog key: it becomes a path component, so reject anything that
  # could escape or nest outside .claude/skills/<name>/.
  case "$name" in
  "" | . | .. | */*)
    echo "[fail] $name: invalid skill name" >&2
    return 1
    ;;
  esac
  url=$(jq -r --arg n "$name" '.[$n] // empty' "$catalog")
  if [ -z "$url" ]; then
    echo "[fail] $name: not in $(basename "$catalog")" >&2
    return 1
  fi
  dir="$skills_dir/$name"
  mkdir -p "$dir"
  tmp=$(mktemp "$dir/.SKILL.md.XXXXXX") || {
    echo "[fail] $name: mktemp" >&2
    return 1
  }
  if curl -fsSL "$url" -o "$tmp" && chmod 644 "$tmp" && mv "$tmp" "$dir/SKILL.md"; then
    echo "[ok] $name -> ${dir#"$root"/}/SKILL.md"
  else
    rm -f "$tmp"
    rmdir "$dir" 2>/dev/null || true # drop the dir if this run created it empty
    echo "[fail] $name: $url" >&2
    return 1
  fi
}

if [ "$#" -ge 1 ]; then
  sync_one "$1"
else
  # Capture keys before the loop so a jq/catalog failure aborts loudly under set -e
  # (a process substitution's exit status would otherwise be invisible).
  names=$(jq -r 'keys[]' "$catalog")
  rc=0
  while IFS= read -r name; do
    [ -n "$name" ] || continue
    sync_one "$name" || rc=1
  done <<<"$names"
  exit "$rc"
fi

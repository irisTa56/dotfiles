#!/usr/bin/env bash
# Push local edits to a gist-sourced skill back to its gist, then pin apm.yml to the pushed commit.
# APM deploys the skill from apm.yml's gist dependency (aliased to the skill's name) at its
# `ref`, and restores that revision on every `apm install`, so a local edit is lost unless it
# is pushed here and the ref moved to it.
# Usage: push_gist_skill.sh <name>
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
manifest="$root/apm.yml"

name="${1:-}"
# Guard the name: it becomes a path component.
case "$name" in
"" | . | .. | */*)
  echo "[fail] ${name:-<empty>}: invalid skill name" >&2
  exit 1
  ;;
esac

# A field of the skill's gist dependency in apm.yml.
declared() {
  N="$name" yq -r ".dependencies.apm[] | select(.alias == strenv(N)) | .$1" "$manifest"
}
url=$(declared git)
ref=$(declared ref)
case "$url" in
https://gist.github.com/*) ;;
*)
  echo "[fail] $name: no gist dependency by that alias in $(basename "$manifest")" >&2
  exit 1
  ;;
esac
if ! printf '%s' "$ref" | grep -Eq '^[0-9a-f]{40}$'; then
  echo "[fail] $name: its gist dependency has no commit ref to push on top of" >&2
  exit 1
fi

file="$root/.claude/skills/$name/SKILL.md"
if [ ! -f "$file" ]; then
  echo "[fail] $name: no local SKILL.md at ${file#"$root"/} (run 'apm install' first)" >&2
  exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
git clone -q "$url" "$tmp"
# Commit on top of the pinned revision, so a gist that has moved past it rejects the push
# as non-fast-forward rather than losing that revision.
git -C "$tmp" reset -q --hard "$ref"
cp "$file" "$tmp/SKILL.md"
git -C "$tmp" commit -qam "Update $name"
# gh supplies the credential for this push alone; the empty helper first drops the
# configured ones, so none of them stores gh's token.
git -C "$tmp" -c credential.helper= -c credential.helper='!gh auth git-credential' push -q
pushed=$(git -C "$tmp" rev-parse HEAD)
echo "[ok] $name -> $url at $pushed"

N="$name" R="$pushed" yq -i '(.dependencies.apm[] | select(.alias == strenv(N)) | .ref) = strenv(R)' "$manifest"
cd "$root"
apm install
echo "[note] commit apm.yml and apm.lock.yaml and land them on main, or an install from main restores the old copy"

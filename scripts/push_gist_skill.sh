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

# repo_url is <owner>/<gist_id> for a gist dependency.
repo=$(locked repo_url)
if ! printf '%s' "$repo" | grep -Eq '^[^/]+/[0-9a-f]+$'; then
  echo "[fail] $name: no gist dependency by that alias in $(basename "$lockfile")" >&2
  exit 1
fi

file="$root/.claude/skills/$name/SKILL.md"
if [ ! -f "$file" ]; then
  echo "[fail] $name: no local SKILL.md at ${file#"$root"/} (run 'apm install' first)" >&2
  exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
git clone -q "https://gist.github.com/$repo.git" "$tmp"
# Commit on top of the pinned revision, so a gist that has moved past it rejects the push
# as non-fast-forward rather than losing that revision.
git -C "$tmp" reset -q --hard "$(locked resolved_commit)"
cp "$file" "$tmp/SKILL.md"
git -C "$tmp" commit -qam "Update $name"
# gh supplies the credential for this push alone; the empty helper first drops the
# configured ones, so none of them stores gh's token.
git -C "$tmp" -c credential.helper= -c credential.helper='!gh auth git-credential' push -q origin HEAD
pushed=$(git -C "$tmp" rev-parse HEAD)
echo "[ok] $name -> gist $repo at $pushed"

# Pin the pushed commit, so the next `apm install` restores the edit rather than reverting it.
cd "$root"
apm update --yes "$name"
# A pin other than the pushed commit would restore another revision on the next install.
if [ "$(locked resolved_commit)" != "$pushed" ]; then
  echo "[fail] $name: pushed $pushed but apm.lock.yaml pins $(locked resolved_commit); rerun 'apm update --yes $name'" >&2
  exit 1
fi
echo "[note] commit apm.lock.yaml and land it on main, or an install from main restores the old copy"

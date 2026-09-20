#!/usr/bin/env bash
# Report whether the shared tasks a repository is running are the ones on dotfiles main.
#
# A `git::` include is pinned by `?ref=`, and mise caches the clone under
# MISE_CACHE_DIR/remote-git-tasks-cache keyed on the URL and that ref alone: once a ref
# is cached it is reused without fetching, so a branch ref never picks up new commits.
# Pinning `?ref=<sha>` instead makes each bump a new cache key, and this check is what
# tells the consuming repository that a bump is due.
#
# MISE_TASK_DIR points at the directory the task was loaded from, which is inside that
# cached clone, so the commit in use is readable straight from it.
set -euo pipefail

remote_url="https://github.com/irisTa56/dotfiles.git"

tasks_dir="${MISE_TASK_DIR:-}"
if [ -z "$tasks_dir" ]; then
  echo "[fail] MISE_TASK_DIR is unset; run this through 'mise run', not directly" >&2
  exit 1
fi

in_use=$(git -C "$tasks_dir" rev-parse HEAD)

if ! latest=$(git ls-remote "$remote_url" refs/heads/main | cut -f1); then
  echo "[fail] could not reach $remote_url" >&2
  exit 1
fi
if [ -z "$latest" ]; then
  echo "[fail] $remote_url has no refs/heads/main" >&2
  exit 1
fi

if [ "$in_use" = "$latest" ]; then
  echo "[ok] shared tasks are at dotfiles main ($in_use)"
  exit 0
fi

# A checkout of dotfiles itself loads these tasks from the working tree, so its own HEAD
# is reported here and differs from main on any branch. That is not a stale pin.
echo "[warn] shared tasks in use: $in_use"
echo "[warn] dotfiles main:       $latest"
echo "[warn] if this repository pins the tasks, update the include to ?ref=$latest"
exit 1

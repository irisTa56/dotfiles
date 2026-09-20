#!/usr/bin/env bash
# Report whether the shared tasks a repository is running are the ones on dotfiles main.
#
# A `git::` include is pinned by `?ref=`, and mise caches the clone under
# MISE_CACHE_DIR/remote-git-tasks-cache keyed on the URL and that ref alone: once a ref
# is cached it is reused without fetching, so a branch ref never picks up new commits.
# Pinning `?ref=<sha>` instead makes each bump a new cache key, and this check is what
# tells the consuming repository that a bump is due.
#
# MISE_TASK_DIR points at the directory the task was loaded from, which for a consumer is
# inside that cached clone, so the commit in use is readable straight from it.
#
# Exit codes: 0 nothing to act on, 1 the pin is behind, 2 the check could not be made.
set -euo pipefail

remote_url="https://github.com/irisTa56/dotfiles.git"

tasks_dir="${MISE_TASK_DIR:-}"
if [ -z "$tasks_dir" ]; then
  echo "[fail] MISE_TASK_DIR is unset; run this through 'mise run', not directly" >&2
  exit 2
fi

# Establish that the tasks came from a clone of this repository before reading a commit
# out of it. git walks up from $tasks_dir to whatever repository encloses it, so a copy
# vendored into some other repository would otherwise report that repository's HEAD as a
# dotfiles commit and name it in the advice below.
if ! origin=$(git -C "$tasks_dir" remote get-url origin 2>/dev/null); then
  echo "[fail] $tasks_dir is not inside a git clone with an 'origin' remote" >&2
  exit 2
fi
case "$origin" in
"$remote_url" | "${remote_url%.git}" | "git@github.com:irisTa56/dotfiles.git") ;;
*)
  echo "[fail] the shared tasks came from $origin, not $remote_url" >&2
  exit 2
  ;;
esac

# dotfiles itself loads these tasks from its own working tree through a local path, which
# carries no `?ref=` to compare or to bump. Its HEAD differs from main on every branch.
project_root="${MISE_PROJECT_ROOT:-}"
if [ -n "$project_root" ]; then
  case "$tasks_dir" in
  "$project_root"/*)
    echo "[ok] the shared tasks come from this repository's working tree; there is no pin to check"
    exit 0
    ;;
  esac
fi

in_use=$(git -C "$tasks_dir" rev-parse HEAD)

# pipefail carries a git failure past cut.
if ! latest=$(git ls-remote "$remote_url" refs/heads/main | cut -f1) || [ -z "$latest" ]; then
  echo "[fail] could not read refs/heads/main from $remote_url" >&2
  exit 2
fi

if [ "$in_use" = "$latest" ]; then
  echo "[ok] the shared tasks are at dotfiles main ($in_use)"
  exit 0
fi

echo "[fail] shared tasks in use: $in_use" >&2
echo "[fail] dotfiles main:       $latest" >&2
echo "[fail] update this repository's include to ?ref=$latest" >&2
exit 1

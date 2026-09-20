#!/usr/bin/env bash
#MISE description="Check whether the shared tasks in use are the ones on dotfiles main"
#
# Why a consumer pins a commit rather than a branch, and what each exit code means, is in
# tasks/README.md under "shared-tasks:check". What this script adds is how it answers:
# $MISE_TASK_DIR is this directory wherever the task was loaded from, so for a consumer it
# sits inside mise's cached clone, and the commit in use is readable straight from it.
#
# Exit codes: 0 nothing to act on, 1 the shared tasks differ from main, 2 it could not tell.
set -euo pipefail

remote_url="https://github.com/irisTa56/dotfiles.git"

tasks_dir="${MISE_TASK_DIR:-}"
project_root="${MISE_PROJECT_ROOT:-}"
if [ -z "$tasks_dir" ] || [ -z "$project_root" ]; then
  echo "[fail] MISE_TASK_DIR and MISE_PROJECT_ROOT are what mise sets; run this through 'mise run'" >&2
  exit 2
fi

# Which repository this is, git answers; whether it is this one, it does not have to. A
# `git::` include is a clone mise made from the URL in the include, and a clone made by hand
# is one its owner chose. Files copied out of here into a repository's own tree are
# indistinguishable from this repository's working tree and are reported below as nothing to
# check; telling them apart is what the origin comparison this script used to carry did, and
# a copy is a fork of how these tasks are managed for the copying side to own.
# git's own line prints above this one, as it does for the fetch below: `rev-parse` fails
# for a directory in no repository, one git declines to open, a config it cannot read, and
# swallowing its stderr would leave this line naming a cause it has not established.
if ! clone_root=$(git -C "$tasks_dir" rev-parse --show-toplevel); then
  echo "[fail] could not read a repository at $tasks_dir" >&2
  exit 2
fi

# dotfiles itself loads these tasks from its own working tree, which carries no `?ref=` to
# compare or to bump; its HEAD differs from main on every branch. Test the clone's root
# against the project root rather than testing the path prefix, so a dotfiles clone that
# merely sits inside a consuming repository — a submodule, or one under vendor/ — is still
# checked rather than being read as that repository's own working tree.
if [ "$clone_root" = "$(cd "$project_root" && pwd -P)" ]; then
  echo "[ok] the shared tasks come from this repository's working tree; there is no pin to check"
  exit 0
fi

in_use=$(git -C "$tasks_dir" rev-parse HEAD)

# The commit, not what it carries. Comparing the `tasks` directory instead would keep a
# consumer quiet through a commit that leaves it alone, which is most of them — but it
# would also decide for them which changes here are worth a pin bump, and the answer to
# that is every one: bumping is a line, and it is what keeps their copy of the
# instructions current as well as their copy of the tasks.
# pipefail carries a git failure past cut.
if ! latest=$(git ls-remote "$remote_url" refs/heads/main | cut -f1) || [ -z "$latest" ]; then
  echo "[fail] could not read refs/heads/main from $remote_url" >&2
  exit 2
fi

if [ "$in_use" = "$latest" ]; then
  echo "[ok] the shared tasks are at dotfiles main ($in_use)"
  exit 0
fi

echo "[fail] the pin is not dotfiles main" >&2
echo "[fail] in use: $in_use" >&2
echo "[fail] main:   $latest" >&2
exit 1

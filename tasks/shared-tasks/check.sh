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
if [ -z "$tasks_dir" ]; then
  echo "[fail] MISE_TASK_DIR is unset; run this through 'mise run', not directly" >&2
  exit 2
fi

# Which repository this is, git answers; whether it is this one, it does not have to. A
# `git::` include is a clone mise made from the URL in the include, and a clone made by hand
# is one its owner chose. Files copied out of here into some other repository would compare
# that repository's commits against ours and report a difference, which is the answer a
# divergence deserves.
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
project_root="${MISE_PROJECT_ROOT:-}"
if [ -n "$project_root" ] && [ "$clone_root" = "$(cd "$project_root" && pwd -P)" ]; then
  echo "[ok] the shared tasks come from this repository's working tree; there is no pin to check"
  exit 0
fi

in_use=$(git -C "$tasks_dir" rev-parse HEAD)

# Fetching by URL rather than from `origin` keeps this off whatever credentials the clone
# was made with, and it leaves the checkout on the commit in use.
if ! git -C "$tasks_dir" fetch --quiet "$remote_url" main; then
  echo "[fail] could not fetch main from $remote_url" >&2
  exit 2
fi
latest=$(git -C "$tasks_dir" rev-parse FETCH_HEAD)

# The library is the clone's `tasks`, named from its root. A pathspec relative to
# "$tasks_dir" would instead name the directory holding this file, so the check would
# compare itself against itself and stay quiet when any other shared task moves. Only
# tasks/README.md is left out, and by name rather than by extension: it is for whoever adds
# a task and a consumer never runs it, while anything else a task reads is theirs to run.
if git -C "$clone_root" diff --quiet "$in_use" "$latest" -- tasks ':(exclude)tasks/README.md'; then
  echo "[ok] the shared tasks match dotfiles main ($latest)"
  exit 0
fi

echo "[fail] the shared tasks differ from dotfiles main" >&2
echo "[fail] in use: $in_use" >&2
echo "[fail] main:   $latest" >&2
exit 1

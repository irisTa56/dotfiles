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

bump=false
case "$*" in
"") ;;
--bump) bump=true ;;
*)
  echo "[fail] unknown arguments '$*'; the one this takes is --bump" >&2
  exit 2
  ;;
esac

remote_url="https://github.com/irisTa56/dotfiles.git"

tasks_dir="${MISE_TASK_DIR:-}"
project_root="${MISE_PROJECT_ROOT:-}"
if [ -z "$tasks_dir" ] || [ -z "$project_root" ]; then
  echo "[fail] MISE_TASK_DIR and MISE_PROJECT_ROOT are what mise sets; run this through 'mise run'" >&2
  exit 2
fi

# A git hook runs with GIT_DIR and its kin exported, which would point every `git -C` below,
# the scratch fetch included, at the repository the hook runs for.
# shellcheck disable=SC2046 # the names are one per line and need splitting
unset $(git rev-parse --local-env-vars)

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
in_use_tree=$(git -C "$tasks_dir" rev-parse HEAD:tasks)

# What the commit carries under `tasks`, not the commit: most commits to main leave it
# alone, and a check that fails on every one of them gets ignored. The README a consumer
# follows sits under `tasks` too, so a change to it still asks for a bump.
# `ls-remote` gives main's commit but not its tree, so fetch main's tip alone, trees
# without blobs, into a throwaway repository; fetching into mise's cached clone instead
# would change a directory mise owns.
if ! scratch=$(mktemp -d); then
  echo "[fail] could not create a scratch directory" >&2
  exit 2
fi
trap 'rm -rf "$scratch"' EXIT
if ! git -C "$scratch" init -q; then
  echo "[fail] could not create a scratch repository at $scratch" >&2
  exit 2
fi
if ! git -C "$scratch" fetch -q --depth=1 --filter=blob:none "$remote_url" refs/heads/main ||
  ! latest=$(git -C "$scratch" rev-parse FETCH_HEAD) ||
  ! latest_tree=$(git -C "$scratch" rev-parse FETCH_HEAD:tasks); then
  echo "[fail] could not read the tasks tree of refs/heads/main from $remote_url" >&2
  exit 2
fi

if [ "$in_use_tree" = "$latest_tree" ]; then
  echo "[ok] the pinned tasks match dotfiles main ($latest)"
  exit 0
fi

echo "[fail] the pinned tasks differ from dotfiles main" >&2
echo "[fail] in use: $in_use" >&2
echo "[fail] main:   $latest" >&2

if [ "$bump" != true ]; then
  exit 1
fi

pin="git::${remote_url}//tasks?ref="
compare="${remote_url%.git}/compare"

# Outside GitHub Actions the move lands in the working tree, for whoever ran it to commit.
if [ "${GITHUB_ACTIONS:-}" != true ]; then
  config="$project_root/mise.toml"
  # Not sed -i, whose form differs between GNU and BSD sed; writing through the file keeps its mode.
  sed "s|${pin}[0-9a-f]\{40\}|${pin}${latest}|" "$config" >"$scratch/mise.toml"
  if cmp -s "$config" "$scratch/mise.toml"; then
    echo "[fail] $config holds no pin of ${pin}<sha> to move, or holds main's already" >&2
    exit 2
  fi
  cat "$scratch/mise.toml" >"$config"
  echo "[fail] moved the pin to main; what it brings: $compare/${in_use}...${latest}" >&2
  exit 1
fi

# In GitHub Actions the checkout is whatever the run was for, often a pull request's merge
# commit, so the move is made on the repository's default branch instead and pushed to a
# branch of its own. A person opens the pull request from it: opening one with the job's
# token needs a repository setting that also lets that token approve pull requests.
bump_branch=shared-tasks-bump

# Once the branch exists it is the person's who opens its pull request, who may add commits
# to it, so it is left alone; merging that pull request deletes it where the repository
# deletes merged branches, and the next stale run pushes afresh.
status=0
git -C "$project_root" ls-remote -q --exit-code origin "refs/heads/$bump_branch" >/dev/null || status=$?
case $status in
0)
  echo "[fail] $bump_branch already exists on origin, so it is left as it is" >&2
  exit 1
  ;;
2) ;;
*)
  echo "[fail] could not tell whether $bump_branch exists on origin" >&2
  exit 2
  ;;
esac

if ! git -C "$project_root" fetch -q --depth=1 origin HEAD ||
  ! base=$(git -C "$project_root" rev-parse FETCH_HEAD) ||
  ! git -C "$project_root" show "$base:mise.toml" >"$scratch/base.toml"; then
  echo "[fail] could not read mise.toml on origin's default branch" >&2
  exit 2
fi
sed "s|${pin}[0-9a-f]\{40\}|${pin}${latest}|" "$scratch/base.toml" >"$scratch/mise.toml"
if cmp -s "$scratch/base.toml" "$scratch/mise.toml"; then
  echo "[fail] mise.toml on origin's default branch holds no pin to move, or holds main's already" >&2
  exit 2
fi
# The move is from the default branch's pin, which may differ from the checkout's.
from=$(sed -n "s|.*${pin}\([0-9a-f]\{40\}\).*|\1|p" "$scratch/base.toml" | head -n 1)
brings="what it brings: $compare/${from}...${latest}"

# Build the commit without touching the working tree or its index: a scratch index over the default branch's
# tree, with mise.toml swapped in at the mode it had there.
mode=$(git -C "$project_root" ls-tree "$base" mise.toml | cut -d' ' -f1)
blob=$(git -C "$project_root" hash-object -w "$scratch/mise.toml")
export GIT_INDEX_FILE="$scratch/index"
git -C "$project_root" read-tree "$base"
git -C "$project_root" update-index --cacheinfo "$mode,$blob,mise.toml"
tree=$(git -C "$project_root" write-tree)
unset GIT_INDEX_FILE
export GIT_AUTHOR_NAME="github-actions[bot]" GIT_COMMITTER_NAME="github-actions[bot]"
export GIT_AUTHOR_EMAIL="41898282+github-actions[bot]@users.noreply.github.com"
export GIT_COMMITTER_EMAIL="$GIT_AUTHOR_EMAIL"
commit=$(git -C "$project_root" commit-tree "$tree" -p "$base" \
  -m "chore(mise): move the shared-tasks pin to dotfiles main" -m "$brings")
if ! git -C "$project_root" push -q origin "$commit:refs/heads/$bump_branch"; then
  echo "[fail] could not push the move to $bump_branch, for the reason git gives above; the push needs the job's token with contents: write kept on the checkout's origin" >&2
  exit 2
fi
echo "[fail] pushed the move to $bump_branch; $brings" >&2
echo "[fail] open its pull request: ${GITHUB_SERVER_URL:-https://github.com}/${GITHUB_REPOSITORY:-}/compare/$bump_branch?expand=1" >&2
exit 1

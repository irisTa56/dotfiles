#!/usr/bin/env bash
# Bring a base branch up to date with origin, typically after a pull request into it merges.
# Usage: sync-base.sh [branch]   # no branch = origin's default branch
#
# Where no other worktree has the branch checked out, this checkout switches to it and
# fast-forwards. Where another worktree has it, git refuses that switch, so the other
# worktree is fast-forwarded in place and this checkout stays on its current branch.
set -euo pipefail

git -c remote.origin.followRemoteHEAD=always fetch origin

branch="${1:-}"
if [[ -z "$branch" ]]; then
  branch=$(git symbolic-ref --short refs/remotes/origin/HEAD)
fi
branch="${branch#origin/}"

holder=$(git worktree list --porcelain |
  awk -v ref="branch refs/heads/$branch" '/^worktree /{p=substr($0,10)} $0==ref{print p}')

current=$(git branch --show-current)
if [[ -n "$holder" && "$current" != "$branch" ]]; then
  git -C "$holder" merge --ff-only "origin/$branch"
  echo "Fast-forwarded $branch in $holder; this checkout stays on ${current:-a detached HEAD}." >&2
else
  git switch "$branch"
  git merge --ff-only "origin/$branch"
fi

#!/usr/bin/env bash
# Fast-forward the default branch, then delete local branches whose upstream is gone.
# Wherever delete-on-merge is enabled, a gone upstream means the pull request merged.
# The only other cause is a branch deleted by hand, and `git branch -D` prints the sha, so the
# reflog restores a branch ref removed in error.
# Nothing restores a removed worktree's ignored files, which git declines to protect, so each
# removal says so before it happens.
# Usage: cleanup_merged_branches.sh   # acts on the current repository
set -euo pipefail

if [ "$#" -gt 0 ]; then
  echo "usage: ${0##*/}" >&2
  exit 2
fi

# Path of the worktree holding $1, empty when no worktree holds it.
# Inside a submodule the main entry names the gitdir rather than the working tree,
# so a hit is normalized through rev-parse; elsewhere that is a no-op.
worktree_of() {
  local hit
  hit=$(git worktree list --porcelain | awk -v want="refs/heads/$1" '
    /^worktree / { path = substr($0, 10) }
    /^branch /   { if (substr($0, 8) == want) { print path; exit } }')
  if [ -n "$hit" ]; then git -C "$hit" rev-parse --show-toplevel; fi
}

git fetch --prune -q
# Drop admin entries for worktrees whose directory is already gone, so nothing below
# resolves a path that does not exist. This removes no work of its own.
git worktree prune

base=$(git symbolic-ref --short refs/remotes/origin/HEAD | sed 's|^origin/||')
base_worktree=$(worktree_of "$base")
# Fetching into a branch fails wherever a worktree holds it, not only in this one.
# Both forms refuse a non-fast-forward, and a base carrying local-only commits is reported
# rather than rewritten. That does not stop the sweep, which does not depend on it.
if [ -n "$base_worktree" ]; then
  git -C "$base_worktree" merge --ff-only "origin/$base" || echo "skipped $base: not a fast-forward"
else
  git fetch origin "$base:$base" || echo "skipped $base: not a fast-forward"
fi

here=$(git rev-parse --show-toplevel)
gone=$(git for-each-ref --format='%(refname:short) %(upstream:track)' refs/heads |
  awk '$2 == "[gone]" { print $1 }')

while IFS= read -r branch; do
  [ -n "$branch" ] || continue
  worktree=$(worktree_of "$branch")
  if [ -n "$worktree" ]; then
    # Removing the worktree this run stands in takes the working directory with it, and git
    # does not refuse that, so it is the one refusal this script has to make for itself.
    if [ "$worktree" = "$here" ]; then
      echo "skipped $branch: rerun from outside $worktree to remove it"
      continue
    fi
    if git -C "$worktree" status --porcelain --ignored | grep -q '^!!'; then
      echo "removing $worktree also deletes the ignored files it holds, which nothing restores"
    fi
    # git states its own reason on stderr, so name the branch here and leave the reason to it.
    if ! git worktree remove "$worktree"; then
      echo "skipped $branch: could not remove $worktree"
      continue
    fi
    echo "removed worktree $worktree"
  fi
  git branch -D "$branch"
done <<<"$gone"

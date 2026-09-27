#!/usr/bin/env bash
#MISE description="Run gh-poi, then delete the local branches it leaves that carry no commits of their own"
#MISE dir="{{cwd}}"
set -euo pipefail

# gh-poi deletes a branch only once it finds a pull request holding the branch's commits,
# so a branch that never got one stays however long ago its tip reached the default branch.
# Here such a branch goes too. A branch `gh poi lock` holds is left alone by both.

git -c remote.origin.followRemoteHEAD=always fetch --prune origin
# A worktree whose directory is gone stays registered, and holds its branch, until pruned.
git worktree prune
gh-poi

base=$(git symbolic-ref --short refs/remotes/origin/HEAD)

# A branch checked out in any worktree cannot be deleted.
checked_out=$(git worktree list --porcelain | sed -n 's|^branch refs/heads/||p')

while read -r branch; do
  [[ $branch == "${base#origin/}" ]] && continue
  grep -qxF "$branch" <<<"$checked_out" && continue
  [[ $(git config --type=bool "branch.$branch.gh-poi-locked" 2>/dev/null) == true ]] && continue
  # -D, since -d checks against the branch's upstream or HEAD rather than $base,
  # and --merged has just shown $base holds every commit on it.
  git branch -D "$branch"
done < <(git for-each-ref --merged "$base" --format='%(refname:short)' refs/heads)

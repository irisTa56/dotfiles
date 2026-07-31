#!/usr/bin/env bash
# Fast-forward the default branch, then delete local branches whose pull request has merged.
# A gone upstream only narrows the candidates: it also covers a branch whose remote copy was
# deleted without merging, where the local ref is the last place that work exists.
# So each candidate is confirmed against a merged pull request, and its tip must still be the
# commit that merged, or work added after the merge would go with it.
# Anything that cannot be confirmed is kept, including a branch with no upstream at all,
# which the gone-upstream filter never offers as a candidate.
# Removing a worktree deletes the ignored files it holds, and nothing restores them.
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
  if [ -n "$hit" ]; then git -C "$hit" rev-parse --show-toplevel 2>/dev/null || printf '%s\n' "$hit"; fi
}

git fetch --prune -q
# Drop admin entries for worktrees whose directory is already gone. This removes no work of its
# own, and it leaves a locked entry behind, which is why the lookup above tolerates a bad path.
git worktree prune

base=$(git symbolic-ref --short refs/remotes/origin/HEAD | sed 's|^origin/||')
base_worktree=$(worktree_of "$base")
# Which form applies, and why each refusal is git's to make, is set out in the
# post-merge-cleanup skill. A base that will not move does not stop the sweep.
if [ -n "$base_worktree" ]; then
  git -C "$base_worktree" merge --ff-only "origin/$base" || echo "skipped $base"
else
  git fetch origin "$base:$base" || echo "skipped $base"
fi

here=$(git rev-parse --show-toplevel)
# The first entry `git worktree list` reports is the main working tree.
main_worktree=$(git worktree list --porcelain | awk '/^worktree /{print substr($0, 10); exit}')
main_worktree=$(git -C "$main_worktree" rev-parse --show-toplevel 2>/dev/null || true)
gone=$(git for-each-ref --format='%(refname:lstrip=2) %(upstream:track)' refs/heads |
  awk '$2 == "[gone]" { print $1 }')

# One call answers every candidate, so an unreachable GitHub is one outcome rather than one per
# branch. The limit is set past the oldest residue a sweep could meet, since a pull request beyond
# it is absent from the answer and its branch would be reported as having none.
merged_prs=$(gh pr list --state merged --limit 1000 \
  --json headRefName,headRefOid --jq '.[] | "\(.headRefName) \(.headRefOid)"' </dev/null) || {
  echo "${0##*/}: gh could not answer, so no branch can be confirmed merged" >&2
  exit 1
}

while IFS= read -r branch; do
  [ -n "$branch" ] || continue
  merged=$(printf '%s\n' "$merged_prs" | awk -v b="$branch" '$1 == b { print $2; exit }')
  if [ -z "$merged" ]; then
    echo "kept $branch: no merged pull request found for it"
    continue
  fi
  if [ "$(git rev-parse "refs/heads/$branch")" != "$merged" ]; then
    echo "kept $branch: it carries commits past the tip that merged"
    continue
  fi
  worktree=$(worktree_of "$branch")
  if [ -n "$worktree" ]; then
    # git never removes a main working tree, so rerunning from elsewhere cannot help; the way
    # out is to switch that checkout off the branch.
    if [ "$worktree" = "$main_worktree" ]; then
      echo "kept $branch: switch $worktree off it first"
      continue
    fi
    # Removing the worktree this run stands in takes the working directory with it, and git
    # does not refuse that, so it is the one refusal this script has to make for itself.
    if [ "$worktree" = "$here" ]; then
      echo "kept $branch: rerun from outside $worktree to remove it"
      continue
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

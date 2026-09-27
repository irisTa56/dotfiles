#!/usr/bin/env bash
#MISE description="Delete the local branches gh-poi leaves behind that carry no commits of their own, and list the rest"
#MISE dir="{{cwd}}"
set -euo pipefail

# gh-poi deletes a branch only once it finds a pull request holding the branch's commits,
# so a branch that never got one stays however long ago its tip reached the default branch.
# Here such a branch goes too, and what neither deletes is listed with what a keep-or-delete
# call rests on. A branch `gh poi lock` holds is left alone by both.

git -c remote.origin.followRemoteHEAD=always fetch --prune origin
gh-poi

base=$(git symbolic-ref --short refs/remotes/origin/HEAD)

# A branch checked out in any worktree cannot be deleted, and is listed instead.
checked_out=$(git worktree list --porcelain | sed -n 's|^branch refs/heads/||p')
locked() { [[ $(git config --type=bool "branch.$1.gh-poi-locked" 2>/dev/null) == true ]]; }

while read -r branch; do
  [[ $branch == "${base#origin/}" ]] && continue
  grep -qxF "$branch" <<<"$checked_out" && continue
  locked "$branch" && continue
  # -D, since -d checks against the branch's upstream or HEAD rather than $base,
  # and --merged has just shown $base holds every commit on it.
  git branch -D "$branch"
done < <(git for-each-ref --merged "$base" --format='%(refname:short)' refs/heads)

echo
echo "Left for a decision (against $base):"
git for-each-ref --format='%(refname:short)' refs/heads | while read -r branch; do
  [[ $branch == "${base#origin/}" ]] && continue
  if locked "$branch"; then
    echo "- $branch: locked"
    continue
  fi
  pr_json=$(gh pr list --head "$branch" --state all --json number,state,headRefOid,mergedAt)
  prs=$(jq -r 'map("#\(.number) \(.state)") | join(", ")' <<<"$pr_json")
  # A squash merge lands a pull request's commits as one, which matches none of them
  # patch for patch once there are several, so only the commits after its head are counted.
  since=$(jq -r 'map(select(.state == "MERGED")) | sort_by(.mergedAt) | last | .headRefOid // empty' <<<"$pr_json")
  counted="in all"
  if [[ -n $since ]] && git merge-base --is-ancestor "$since" "$branch" 2>/dev/null; then
    counted="after merged PR head ${since:0:7}"
  else
    since=
  fi
  # git cherry marks a commit `-` where $base holds an equivalent patch.
  cherry=$(git cherry -v "$base" "$branch" ${since:+"$since"})
  missing=$(grep -c '^+' <<<"$cherry" || true)
  landed=$(grep -c '^-' <<<"$cherry" || true)
  line="- $branch: $missing commit(s) not on $base, $landed landed as equivalent patches ($counted); PRs: ${prs:-none}"
  worktree=$(git worktree list --porcelain |
    awk -v ref="branch refs/heads/$branch" '/^worktree /{p=substr($0,10)} $0==ref{print p}')
  if [[ -n $worktree ]]; then
    dirty=$(git -C "$worktree" status --porcelain | wc -l | tr -d ' ')
    line+="; checked out in $worktree ($dirty uncommitted path(s))"
  fi
  echo "$line"
  grep '^+' <<<"$cherry" | sed 's/^/    /' || true
done

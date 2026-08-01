#!/usr/bin/env bash
# Account for every local branch, and delete the settled ones with -D.
# Deletable means the pull request merged or closed, the remote branch gone on origin, the local
# tip still the commit that pull request carried, and no worktree holding it that cannot go.
# A branch still published on origin is reported with that copy's date instead, since deleting
# only the local side of it is not this script's call.
# Every other branch is reported with the reason it stays, so silence never stands for a branch
# that was skipped rather than judged. The default branch is the one exclusion, and gets no row.
# The whole list prints before -D removes anything, and nothing is judged twice, so the list is
# what -D attempts. A refusal this cannot see coming, such as a locked worktree or a worktree
# holding an initialized submodule, is reported when the removal is attempted.
# Usage: stale_branches.sh [-D]   # -D deletes the listed branches and their worktrees
set -euo pipefail

delete=0
case "$#:${1:-}" in
"0:") ;;
"1:-D") delete=1 ;;
*)
  echo "usage: ${0##*/} [-D]" >&2
  exit 2
  ;;
esac

# The working tree $1 names, unchanged when $1 is empty.
# Inside a submodule a worktree path names the gitdir rather than the working tree, so it is
# normalized, and a stale entry naming a directory that is gone is passed through rather than
# aborting the run. Those entries are never pruned: without them `git worktree repair` cannot
# put a moved worktree back.
normalize_worktree() {
  [ -n "$1" ] || return 0
  git -C "$1" rev-parse --show-toplevel 2>/dev/null || printf '%s\n' "$1"
}

# Why $1 will not be removed, empty when it will be.
worktree_blocker() {
  if [ "$1" = "$main_worktree" ]; then
    echo "it is checked out in the main working tree"
  elif [ "$1" = "$here" ]; then
    echo "this run stands in its worktree"
  elif [ -n "$(git -C "$1" status --porcelain 2>/dev/null)" ]; then
    echo "its worktree has uncommitted changes"
  fi
}

# The branch column comes last, since a branch name is the one field with no useful width.
# Removing a worktree takes its ignored files with it, and git's refusal does not cover those,
# so the listing is the last place they can be noticed.
report() {
  local note=""
  printf '%-7s %-6s %s\n' "$1" "$2" "$3"
  [ -z "${4:-}" ] || {
    [ "$1" != delete ] || [ -z "$(git -C "$4" ls-files --others --ignored --exclude-standard --directory 2>/dev/null)" ] ||
      note=' — removal deletes the ignored files it holds'
    printf '%-7s %-6s wt: %s%s\n' '' '' "$4" "$note"
  }
}

# The refs and the default branch below are origin's, so the fetch has to be origin's too.
# The pull requests are whichever repository gh resolves, which prefers an upstream remote.
if ! git remote get-url origin >/dev/null 2>&1; then
  echo "${0##*/}: not a git repository with an origin remote" >&2
  exit 1
fi

git fetch --prune -q origin

here=$(git rev-parse --show-toplevel)
# The first entry `git worktree list` reports is the main working tree, which git never removes.
main_worktree=$(git worktree list --porcelain | awk '/^worktree / && !seen { seen = 1; main = substr($0, 10) } END { print main }')
main_worktree=$(normalize_worktree "$main_worktree")
default_branch=$(git symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null | sed 's|^origin/||') || true
if [ -z "$default_branch" ]; then
  echo "${0##*/}: no origin/HEAD, so this repository is not one it can judge" >&2
  exit 1
fi

# `--state all` is what makes an open pull request distinguishable from none at all, and it puts
# the newest pull request for a branch first, which is the one that says where the branch stands.
# The limit bounds pull requests, not branch age: a branch whose pull request falls outside the
# newest 1000 is reported as having none. Where gh resolves an upstream rather than a personal
# remote, the answer also carries every contributor's branches, so a common local name can match
# a stranger's pull request; the tip comparison then keeps the branch, with the wrong reason.
prs=$(gh pr list --state all --limit 1000 \
  --json state,headRefName,headRefOid \
  --jq '.[] | "\(.headRefName) \(.state) \(.headRefOid)"' </dev/null) || {
  echo "${0##*/}: gh could not answer, so no branch can be judged" >&2
  exit 1
}

# The last --sort key is the primary one, so a branch some worktree holds — the only kind that
# reports on two lines — comes first, and the rest stay in name order.
branches=$(git for-each-ref --sort=refname --sort=-worktreepath \
  --format='%(refname:lstrip=2) %(worktreepath)' refs/heads)

deletable=""
while read -r branch worktree; do
  [ -n "$branch" ] || continue
  [ "$branch" != "$default_branch" ] || continue
  worktree=$(normalize_worktree "$worktree")
  pr=$(awk -v b="$branch" '$1 == b { print $2 " " $3; exit }' <<<"$prs")
  if [ -z "$pr" ]; then
    report keep - "$branch, no pull request" "$worktree"
    continue
  fi
  state=$(printf '%s' "${pr%% *}" | tr '[:upper:]' '[:lower:]')
  if [ "$state" = "open" ]; then
    report keep "$state" "$branch, its pull request is still open" "$worktree"
  elif git show-ref --verify --quiet "refs/remotes/origin/$branch"; then
    report keep "$state" \
      "$branch, still published, updated $(git log -1 --format=%cs "refs/remotes/origin/$branch")" "$worktree"
  elif [ "$(git rev-parse "refs/heads/$branch")" != "${pr##* }" ]; then
    report keep "$state" "$branch, tip is not the commit the pull request carried" "$worktree"
  elif [ -n "$worktree" ] && {
    blocker=$(worktree_blocker "$worktree")
    [ -n "$blocker" ]
  }; then
    report keep "$state" "$branch, $blocker" "$worktree"
  else
    report delete "$state" "$branch" "$worktree"
    deletable="$deletable$branch $worktree"$'\n'
  fi
done <<<"$branches"

[ "$delete" -eq 1 ] || exit 0

while read -r branch worktree; do
  [ -n "$branch" ] || continue
  # git states its own reason on stderr, so name the branch here and leave the reason to it.
  if [ -n "$worktree" ] && ! git worktree remove "$worktree"; then
    echo "skipped $branch: could not remove $worktree" >&2
    continue
  fi
  # Unguarded, one failure here would abandon the rest of a plan the user has already read.
  git branch -D "$branch" || true
done <<<"$deletable"

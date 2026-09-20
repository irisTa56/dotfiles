#!/usr/bin/env bash
#MISE description="Scan the commits a push is about to send for secrets"
#MISE tools={ trufflehog = "latest" }
set -euo pipefail

# A pre-push hook's task, and nothing else: git names the remote in the first
# argument and feeds one line per pushed ref on stdin, neither of which a task run
# by hand has.
#
# secrets:scan gates each commit with gitleaks. This is for what gitleaks' default
# rules and GitHub's push protection both miss, a credential embedded in a
# connection string or a URL, and it is paid once per push rather than per commit.
remote="${1:-origin}"

# The repository, not the working directory, for the reason secrets:scan gives.
repo="file://$(git rev-parse --show-toplevel)"
status=0

# <local ref> <local sha> <remote ref> <remote sha>
while read -r local_ref local_sha _remote_ref _remote_sha; do
  # A deletion arrives as `(delete)` with an all-zero local sha, and adds nothing.
  [[ "$local_sha" =~ [^0] ]] || continue

  # The oldest by date of what this ref adds beyond every ref the remote is known
  # to have. Date order rather than topology, because trufflehog walks back from
  # the branch tip and stops at `--since-commit` by commit date: a topic branch
  # cut earlier and merged in has to be inside the range, and taking the remote's
  # tip as the base would leave it out.
  oldest="$(git rev-list --date-order --reverse "$local_sha" --not --remotes="$remote" | sed -n '1p')"

  # Naming an already-pushed commit as a new branch sends nothing to scan, and
  # scanning anyway would answer for what the remote already holds.
  [ -n "$oldest" ] || continue

  # An orphan branch and a remote with no history leave that commit without a
  # parent to start from, and then the whole branch is the range rather than a
  # reason to refuse the push.
  if base="$(git rev-parse --verify --quiet "$oldest^")"; then
    since=(--since-commit "$base")
  else
    since=()
  fi

  # `--trust-local-git-config` is left off: with it trufflehog reads the repository
  # with go-git, which rejects a config that sets `extensions.worktreeConfig`, as
  # this machine's repositories do. Verification stays off so a push never sends a
  # candidate to its provider.
  trufflehog git "$repo" \
    --branch "${local_ref#refs/heads/}" \
    ${since[@]+"${since[@]}"} \
    --no-verification --fail --fail-on-scan-errors --no-update --concurrency=1 ||
    status=$?
done

exit "$status"

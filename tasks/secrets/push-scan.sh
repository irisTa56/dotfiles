#!/usr/bin/env bash
#MISE description="Scan the commits a push is about to send for secrets"
#MISE tools={ trufflehog = "latest", jq = "latest" }
set -euo pipefail

# A pre-push hook's task, and nothing else: git names the remote in its first
# argument and feeds one line per pushed ref on stdin. A caller that passes no
# argument stops here on it; one that passes the argument and no ref lines cannot
# be told apart from an up-to-date push, which git runs this for with an empty
# stdin, so it scans nothing and passes.
#
# secrets:scan gates each commit with gitleaks. This is for the credentials
# trufflehog has a detector for and gitleaks' default rules do not, and it is paid
# once per push rather than once per commit.
remote="$1"

# The repository, not the working directory, for the reason secrets:scan gives.
repo="file://$(git rev-parse --show-toplevel)"
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
status=0

# <local ref> <local sha> <remote ref> <remote sha>
while read -r local_ref local_sha _remote_ref _remote_sha; do
  # A deletion arrives as `(delete)` with an all-zero local sha, and adds nothing.
  [[ "$local_sha" =~ [^0] ]] || continue

  # What this ref would send, oldest by date first. Nothing means the remote holds
  # all of it already, and scanning would answer for what it already has.
  new="$(git rev-list --date-order --reverse "$local_sha" --not --remotes="$remote")"
  [ -n "$new" ] || continue

  # trufflehog walks back from the branch tip and stops at `--since-commit` by
  # commit date, so only a base older than every one of those commits keeps them
  # all in range. The parent of the oldest is that. It is a bound and not an
  # answer: the range also reaches commits the remote already has, which is what
  # the filter below drops. An orphan branch and a remote with no history leave
  # that commit without a parent, and then the whole branch is the range.
  if base="$(git rev-parse --verify --quiet "$(printf '%s\n' "$new" | sed -n '1p')^")"; then
    since=(--since-commit "$base")
  else
    since=()
  fi

  # `--trust-local-git-config` is left off: with it trufflehog reads the repository
  # with go-git, which rejects a config that sets `extensions.worktreeConfig`, as
  # this machine's repositories do. Verification stays off so a push never sends a
  # candidate to its provider.
  #
  # What reaches the terminal is this task's own report and not trufflehog's, which
  # counts every hit in the range and so would announce a secret on a push the
  # filter above lets through. Its log is held back and printed when the scan
  # itself failed, which is the one time it says something this cannot.
  if ! hits="$(trufflehog git "$repo" \
    --branch "${local_ref#refs/heads/}" \
    ${since[@]+"${since[@]}"} \
    --json --no-verification --fail-on-scan-errors --no-update --concurrency=1 2>"$log" |
    jq -r --arg new "$new" '
      ($new | split("\n")) as $sending
      | select(.SourceMetadata.Data.Git.commit | IN($sending[]))
      | "\(.DetectorName) at line \(.SourceMetadata.Data.Git.line)"
        + " of \(.SourceMetadata.Data.Git.file // "the commit message")"
        + ", commit \(.SourceMetadata.Data.Git.commit)"')"; then
    cat "$log" >&2
    exit 1
  fi

  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" >&2
    status=1
  fi
done

exit "$status"

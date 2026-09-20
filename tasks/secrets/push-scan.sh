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

  # trufflehog stops at `--since-commit` by commit date rather than by ancestry, so
  # a base holds the set only while it is older than the oldest of them. The parent
  # of that one usually is, and is checked rather than assumed, since a committer
  # date that falls along a parent edge would leave its own child out of the range.
  # Where it does, and where an orphan branch or a remote with no history leaves no
  # parent at all, the whole branch is the range instead. Either way this bounds the
  # walk rather than answering it: the range still reaches commits the remote has,
  # which is what the filter below drops.
  oldest="$(printf '%s\n' "$new" | sed -n '1p')"
  since=()
  if base="$(git rev-parse --verify --quiet "$oldest^")" &&
    [ "$(git log -1 --format=%ct "$base")" -lt "$(git log -1 --format=%ct "$oldest")" ]; then
    since=(--since-commit "$base")
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
    --json --no-verification --fail-on-scan-errors --no-update 2>"$log" |
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

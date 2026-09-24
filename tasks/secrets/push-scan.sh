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
# secrets:commit-scan gates each commit with gitleaks. This is the second pass its default
# rules do not give, paid once per push rather than once per commit. What it reaches
# is answered in tasks/README.md and is not summarisable here.
remote="$1"

# The repository, not the working directory, for the reason secrets:commit-scan gives.
repo="file://$(git rev-parse --show-toplevel)"
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
status=0

# <local ref> <local sha> <remote ref> <remote sha>
while read -r _local_ref local_sha _remote_ref _remote_sha; do
  # A deletion arrives as `(delete)` with an all-zero local sha, and adds nothing.
  [[ "$local_sha" =~ [^0] ]] || continue

  # What this ref would send. Nothing means the remote holds all of it already, and
  # scanning would answer for what it already has.
  new="$(git rev-list "$local_sha" --not --remotes="$remote")"
  [ -n "$new" ] || continue

  # The commit git is pushing, peeled so an annotated tag gives its commit. The walk
  # starts here rather than at the ref's name, which trufflehog would resolve again
  # in its own clone, where a tag of the same name wins over the branch.
  tip="$(git rev-parse "$local_sha^{commit}")"

  # Everything behind it is walked, and the filter below, on the set above, decides
  # what is reported. `--since-commit` with a commit the remote already holds would
  # shorten the walk without changing that answer; it is not passed, and a remote
  # that holds nothing yet offers no such commit.
  #
  # `--trust-local-git-config` is left off: with it trufflehog reads the repository
  # with go-git, which rejects a config that sets `extensions.worktreeConfig`, as
  # this machine's repositories do. Verification stays off so a push never sends a
  # candidate to its provider.
  #
  # What reaches the terminal is this task's own report and not trufflehog's, which
  # counts every hit it walked and so would announce a secret on a push the filter
  # below lets through. Its log is held back and printed when the scan itself
  # failed, which is the one time it says something this cannot.
  if ! hits="$(trufflehog git "$repo" \
    --branch "$tip" \
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

#!/bin/sh
# PreToolUse hook for Bash: block a `git push` chained with && after a piped
# git merge/rebase/pull/commit. Without pipefail, && tests the last pipe stage,
# so `git merge --ff-only x | tail -1 && git push` pushes even when the merge
# failed. Exit 2 blocks the call and hands stderr back to Claude.
# Matching is per line: a line that sets pipefail is skipped, and a chain split
# across lines is not caught.

pattern='git[^|;]* (merge|rebase|pull|commit)[^|;]*\|[^;]*&&[^;]*git[^;]* push'

if jq -r '.tool_input.command // empty' | grep -v pipefail | grep -qE "$pattern"; then
  echo 'Blocked: the git merge/rebase/pull/commit is piped, so && tests the last pipe stage and the push would run even if that step failed. Drop the pipe, or push in a separate command after checking the result.' >&2
  exit 2
fi

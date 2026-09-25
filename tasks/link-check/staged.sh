#!/usr/bin/env bash
#MISE description="Check the links of staged files over the network"
#MISE tools={ lychee = "latest" }
set -euo pipefail

# Every link in a staged file is requested, old ones included, so a host failing on
# one of those blocks the commit too; a link in an unstaged file blocks nothing.
# --cache skips links that passed within the last day; lychee never caches a failure.

# The repository root, not the working directory, for the reason secrets:commit-scan
# gives; lychee also reads lychee.toml and writes .lycheecache there.
cd "$(git rev-parse --show-toplevel)"

# The staged files among those `lychee .` reads, so the repository's lychee config and
# .gitignore decide which files count: a file named on the command line is checked
# whatever its extension and whether or not it is ignored.
files=$(
  lychee --dump-inputs . | sed 's|^\./||' | {
    # grep exits 1 when no staged file is among them, which is not an error. git quotes
    # a non-ASCII name unless told not to, and lychee prints it raw.
    grep -Fx -f <(git -c core.quotePath=false diff --cached --name-only --diff-filter=d) || [ $? -eq 1 ]
  }
)
[ -n "$files" ] || exit 0

printf '%s\n' "$files" | tr '\n' '\0' | xargs -0 lychee --cache --no-progress || {
  # The bypass at hand otherwise is --no-verify, which drops every other check with it.
  # The skip takes the task's full name, `.sh` included; the bare name skips nothing.
  echo "If a failing link is not one this commit adds, skip only this check: MISE_TASK_SKIP=${MISE_TASK_NAME:-link-check:staged.sh} git commit ..." >&2
  exit 1
}

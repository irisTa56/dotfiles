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
# whatever its extension and whether or not it is ignored. git does the matching,
# taking lychee's list as literal pathspecs, so a name git stores in another Unicode
# form than the one on disk (macOS precomposes) still matches; -r keeps git from
# listing every staged file when lychee lists none.
files=$(
  lychee --dump-inputs . | sed 's|^\./||' | tr '\n' '\0' |
    xargs -0 -r git --literal-pathspecs -c core.quotePath=false diff --cached --name-only --diff-filter=d --
)
[ -n "$files" ] || exit 0

# lychee expands an input as a glob and reads one starting with `-` as an option, so
# `[`, `*` and `?` are bracketed to match only themselves, and each path gets `./`.
printf '%s\n' "$files" | sed -e 's/[*?[]/[&]/g' -e 's|^|./|' | tr '\n' '\0' |
  xargs -0 lychee --cache --no-progress || {
  # The bypass at hand otherwise is --no-verify, which drops every other check with it.
  # The skip takes the task's full name, `.sh` included; the bare name skips nothing.
  echo "If a failing link is not one this commit adds, skip only this check: MISE_TASK_SKIP=${MISE_TASK_NAME:-link-check:staged.sh} git commit ..." >&2
  exit 1
}

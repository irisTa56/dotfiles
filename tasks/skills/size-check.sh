#!/usr/bin/env bash
#MISE description="Fail where a tracked skill's body is past what a compaction re-attaches"
set -euo pipefail

# After a compaction, Claude Code re-attaches each skill the session invoked, cut
# short: its docs say to the first 5,000 tokens, and session transcripts show the
# cut at 20,000 characters. A rule past the cut is gone from the re-attached copy.
# Length is counted in UTF-16 code units, as JavaScript counts a string's, which is
# never fewer than its characters. The re-attached copy opens with a line naming the
# skill's directory, whose length depends on where the skill was loaded from.
limit=$((20000 - 200))

# Asked of git, so an ignored or vendored skill is left out and one staged for the
# commit being made is checked. The argument names another layout; `glob` keeps
# `*` to one directory level.
pathspec=${1:-':(glob).claude/skills/*/SKILL.md'}

files=$(git ls-files -- "$pathspec")
# Run where git lists nothing, the check would otherwise pass having read no skill.
if [[ -z $files ]]; then
  echo "git tracks no skill at $pathspec to check." >&2
  exit 1
fi

status=0
while IFS= read -r file; do
  # The body without its frontmatter, which is what the copy holds.
  length=$(($(sed '1,/^---$/d' "$file" | iconv -f UTF-8 -t UTF-16LE | wc -c) / 2))
  if ((length > limit)); then
    echo "$file: $length" >&2
    status=1
  fi
done <<<"$files"
exit "$status"

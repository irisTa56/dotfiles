#!/usr/bin/env bash
#MISE description="Fail where a tracked skill's body is past what a compaction re-attaches; --verbose prints each passing skill's length too"
set -euo pipefail

# After a compaction, Claude Code re-attaches each skill the session invoked, cut
# short: its docs say to the first 5,000 tokens, and session transcripts show the
# cut at 20,000 characters. A rule past the cut is gone from the re-attached copy.
# Length is counted in UTF-16 code units, as JavaScript counts a string's, which is
# never fewer than its characters. The re-attached copy opens with a line naming the
# skill's directory, whose length depends on where the skill was loaded from.
limit=$((20000 - 200))

# --verbose also prints the length of each skill that passes, for a session sizing
# a skill against the limit rather than gating a commit on it.
verbose=0
if [[ ${1:-} == --verbose ]]; then
  verbose=1
  shift
fi

# Asked of git, so an ignored or vendored skill is left out and one staged for the
# commit being made is checked. The argument names another layout; `glob` keeps
# `*` to one directory level.
pathspec=${1:-':(glob).claude/skills/*/SKILL.md'}
# An argument past the pathspec, such as a --verbose written last, would otherwise
# be dropped and leave a silent pass.
if (($# > 1)); then
  echo "size-check.sh does not read ${*:2}; its arguments are [--verbose] [pathspec], in that order." >&2
  exit 2
fi

# -z, since git otherwise quotes a path that is not ASCII.
files=$(git ls-files -z -- "$pathspec" | tr '\0' '\n')
# Run where git lists nothing, the check would otherwise pass having read no skill.
if [[ -z $files ]]; then
  echo "git tracks no skill at $pathspec to check." >&2
  exit 1
fi

status=0
while IFS= read -r file; do
  # Deleted from the working tree, which is what is read, and not yet staged.
  [[ -f $file ]] || continue
  # The body without its frontmatter, which is what the copy holds: the lines through
  # the first `---` after one on line 1, or none where line 1 is not `---`. A fence
  # with no close fails rather than taking the whole file for frontmatter.
  fence=$(awk '
    NR == 1 { if ($0 != "---" && $0 != "---\r") { end = 0; exit } next }
    $0 == "---" || $0 == "---\r" { end = NR; exit }
    END { print (end == "" && NR > 0) ? -1 : end + 0 }' "$file")
  if ((fence < 0)); then
    echo "$file: its frontmatter has no closing ---" >&2
    status=1
    continue
  fi
  length=$(($(tail -n "+$((fence + 1))" "$file" | iconv -f UTF-8 -t UTF-16LE | wc -c) / 2))
  if ((length > limit)); then
    echo "$file: $length UTF-16 code units, past the $limit a compaction re-attaches" >&2
    status=1
  elif ((verbose)); then
    echo "$file: $length of $limit UTF-16 code units"
  fi
done <<<"$files"
exit "$status"

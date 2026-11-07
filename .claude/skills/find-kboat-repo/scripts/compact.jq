# One line per repo note, carrying only what judging fit for a need reads.
# kboat-note returns a flow-style list such as `[robotics, ml]` as a string,
# and an empty one as an array, so both shapes become an array here.
def items:
  if type == "array" then .
  elif . == null or . == "" then []
  else ltrimstr("[") | rtrimstr("]") | split(",") | map(gsub("^\\s+|\\s+$"; "") | select(. != ""))
  end;

.notes[] | .frontmatter as $f | {
  title: $f.title,
  role: $f.role,
  domain: ($f.domain | items),
  topics: ($f.topics | items),
  description: $f.description,
  summary: $f.summary
}

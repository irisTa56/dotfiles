---
type: regex
pattern: '\bnit(s|pick|picks|picking)?\b|\bminor (issue|point|concern|finding|nit|suggestion)s?\b|(\[|\()(minor|low|trivial|optional)(\]|\))|\bseverity\W{0,6}(low|minor|trivial)\b|\blow priority\b'
flags: i
match: not_contains
---

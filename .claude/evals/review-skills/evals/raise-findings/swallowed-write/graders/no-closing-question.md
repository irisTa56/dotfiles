---
type: regex
pattern: '\b(would you like me to|do you want me to|want me to|shall I|should I)\b[^.?\n]*\b(fix|apply|address|change|make|update|go ahead)[^.?\n]*\?|(which|what)[^.?\n]{0,40}(would you like|should I|do you want)[^.?\n]*\?'
flags: i
match: not_contains
---

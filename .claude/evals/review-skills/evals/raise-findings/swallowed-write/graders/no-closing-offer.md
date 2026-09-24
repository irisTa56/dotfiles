---
type: llm
---

The skill forbids closing a review by offering to fix, or asking which findings to fix, since that decision belongs to whoever invoked the review.

PASS if the reply ends without offering to fix or apply anything and without asking which findings to act on.

FAIL if the reply offers to fix, apply, or address findings, or asks which to act on, whether as a question ("Want me to apply these?") or a statement ("If you want me to apply any of these, say which.").

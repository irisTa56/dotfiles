---
type: llm
---

The finding reads the first sentence alone. The very next sentence says when to skip the step, so a reader of the section does not run migrations on every release.

PASS if the reply rejects the finding because the following sentence already states the condition, and keeps the text as it is.

FAIL if the reply accepts the rewrite, proposes any other rewording of the section, or accepts the finding in part.

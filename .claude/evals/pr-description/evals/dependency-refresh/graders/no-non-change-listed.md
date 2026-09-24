---
type: llm
---

The author left eslint pinned at `<9`, unchanged. That is a decision not to change something, not a change.

Decide by where eslint is mentioned, not by how it is worded.

PASS if the body does not mention eslint, or mentions it only in a section whose heading marks it as outside the change (such as "Notes", "Out of scope", or "Not included").

FAIL if eslint is mentioned anywhere inside a section that describes what the PR changes (such as "Changes", "Dependency updates", or "Summary"), whether as a bullet, a table row, or a sentence.

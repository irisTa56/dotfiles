---
type: llm
---

The change adds `ignore = ["B008", "E501"]` to the ruff config, though ruff reports nothing on the new code without it.

PASS if the reply raises the ignore line as a finding: it suppresses rules nothing has tripped, so a later violation goes unreported.

FAIL if the reply does not raise it, or mentions it only in passing without asking for it to be removed.

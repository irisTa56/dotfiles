---
type: llm
---

The change adds `httpx>=0.27,<0.28`, an upper bound on a pre-1.0 dependency with no observed break behind it.

PASS if the reply raises the `<0.28` upper bound as a finding and asks for it to be dropped or loosened.

FAIL if the reply does not raise the upper bound, or endorses it.

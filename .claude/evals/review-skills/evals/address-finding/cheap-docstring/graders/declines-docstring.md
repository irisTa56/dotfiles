---
type: llm
---

The finding asks for a docstring on a two-line private helper whose name and body already say what it does. It names nothing that goes wrong without one, and argues only that the addition is cheap.

PASS if the reply declines to add the docstring.

FAIL if the reply accepts the finding, says it would add the docstring, or recommends adding it as optional.

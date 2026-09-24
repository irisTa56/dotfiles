---
type: llm
---

Count only what the reply presents as a finding. A finding holds by naming an input or condition and the wrong outcome it produces.

PASS if every finding names such a wrong outcome.

FAIL if any finding asks for a change without naming what goes wrong under the current code, such as a docstring, a type hint, a rename, a constant, a log line, or tests requested only because they are missing.

---
type: llm
---

PASS if the response names a run that would settle the question: one that varies only the cache layer (with the runtime held the same on both sides), with both sides measured in the same session or sequence.

FAIL if the response proposes no such run, or proposes one that still changes the runtime and the cache layer together, or still compares against last Tuesday's numbers.

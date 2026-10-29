---
type: llm
---

The pool 20 numbers in the notes come from last Thursday, before staging was upgraded to v2.14, which changed the JSON serializer.

PASS if the plan runs pool 20 again as part of this test, on the same build and in the same session as the pool 40 runs, and compares pool 40 against that.

FAIL if the plan compares pool 40 against last Thursday's 180 ms, or leaves it optional whether pool 20 is re-run.

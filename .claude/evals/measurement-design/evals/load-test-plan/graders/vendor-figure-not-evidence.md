---
type: llm
---

The author's notes rely on the database vendor's datasheet to say the database side won't be the bottleneck.

PASS if the plan does not take the datasheet as settling that, and instead measures something on the database side during the test, such as its active connection count, connection waits, or CPU.

FAIL if the plan states or assumes, on the strength of the datasheet, that the database will not be the bottleneck, and measures nothing on the database side.

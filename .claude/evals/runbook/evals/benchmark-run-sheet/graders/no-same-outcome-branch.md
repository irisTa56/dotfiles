---
type: llm
---

In the author's notes, the p99 result leads to the same action whether it is over 50 ms or not: record it in the results table and go on.

PASS if the run sheet gives that step as one instruction (record p99 and go on) with no condition on the 50 ms threshold.

FAIL if the run sheet presents the 50 ms comparison as a branch or condition the executor must evaluate, in prose, a table, or a script, when both outcomes lead to the same action.

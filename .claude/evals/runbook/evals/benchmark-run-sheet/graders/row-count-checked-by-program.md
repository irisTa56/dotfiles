---
type: llm
---

The author's notes ask the executor to check by eye that each CSV has 1,200,000 rows.

PASS if the run sheet checks the row count with a command that compares it against the expected count, and the step that uses the CSV (computing p99) runs only when that comparison succeeds — chained with `&&` on an inline check or on a script that exits non-zero on a mismatch — and a failed comparison prints what it compared (the expected and the actual count).
Printing the count so the executor can record it in the results table is fine.

FAIL if the executor has to judge the count or decide whether to go on: the run sheet asks them to look at, count, or compare the rows themselves, or to read a printed count or verdict (such as `OK` or `MISMATCH`) and choose the next step from it; or the p99 step runs regardless of the comparison; or the row-count check is dropped.

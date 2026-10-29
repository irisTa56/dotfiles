---
type: llm
---

The author's notes ask to try a `--turbo` flag that exists only in the test build.

PASS if the plan leaves the `--turbo` run out of the comparison that decides the production pool size, whether it drops the run or keeps it apart and says it cannot inform that decision.

FAIL if the plan includes a `--turbo` run as a condition that bears on the production decision.

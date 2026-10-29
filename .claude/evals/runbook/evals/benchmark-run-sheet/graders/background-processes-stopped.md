---
type: llm
---

The notes start the API in the background with `&` for each cache size.

PASS if the run sheet stops the API started for one cache size before or as part of starting the next, and stops the last one at the end.

FAIL if any API process the run sheet starts is never stopped by a step on the page.

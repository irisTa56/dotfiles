---
type: llm
---

The body is written for an automated AI reviewer that has no context beyond the PR.
Judge only the title and body; ignore any note to the author after the body.

PASS if both of these hold:

- Where the body first names `WEBHOOK_MAX_RETRIES`, the same sentence or the next one says it sets the maximum number of retries.
- Where the body first names `webhook_delivery_failures_total`, the same sentence or the next one says what the counter counts.

A body that never names one of them passes on that one.

FAIL if either name appears first without that explanation.

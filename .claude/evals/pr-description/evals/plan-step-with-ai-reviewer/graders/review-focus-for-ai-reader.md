---
type: llm
---

The body has a "Review focus" section that an automated AI reviewer reads with no context beyond the PR.
The author wanted the reviewer to look at whether the production failure rate will drop below 1 %, which the diff cannot show, and whether the backoff ever exceeds the retry cap, which the diff can.
Judge only the title and body; ignore any note to the author after the body.

A question about the code, such as an off-by-one in the retry count, an upper bound on the delay, or when the counter is incremented, is one the diff can answer.

PASS if the body has a review-focus section and all of these hold:

- No item in it asks the reviewer to judge production behavior, a future failure rate, or what a load test would show.
- Where the body first names `WEBHOOK_MAX_RETRIES`, the same sentence or the next one says it sets the maximum number of retries.
- Where the body first names `webhook_delivery_failures_total`, if it names it at all, the same sentence or the next one says what the counter counts.

FAIL otherwise.

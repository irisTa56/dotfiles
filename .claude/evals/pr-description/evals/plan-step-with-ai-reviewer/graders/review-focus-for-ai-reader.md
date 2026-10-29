---
type: llm
---

The body has a "Review focus" section that an automated AI reviewer reads with no context beyond the PR.
The author wanted the reviewer to look at whether the production failure rate will drop below 1 %, which the diff cannot show, and whether the backoff ever exceeds the retry cap, which the diff can.
Judge only the title and body; ignore any note to the author after the body.

A question about the code, such as an off-by-one in the retry count, an upper bound on the delay, or when the counter is incremented, is one the diff can answer.
PASS if all of these hold:

- The body has a review-focus section, and no item in it asks the reviewer to judge production behavior, a future failure rate, or what a load test would show.
- The body still states, outside the review focus, that the effect on the production failure rate has not been checked or is not yet known.

FAIL if any of them does not hold.

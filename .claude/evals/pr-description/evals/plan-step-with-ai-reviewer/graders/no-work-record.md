---
type: llm
---

The response is a pull request title and body, possibly followed by a note to the author about how the body was written.
Judge only the title and body; ignore such a note.

The author's notes offered records of the work rather than of the change:

- load-test figures (10,000 deliveries, failure rate 3.1 % to 0.4 %)
- the session's cost (about $3) and the estimate for the next step (about one more day)
- the progress history (a first attempt with a fixed 2-second delay that was reverted, and the two sessions it took)

PASS if the title and body carry none of these.
Saying that a check was run, without its figures (for example "unit tests cover the backoff schedule and the cap, and a staging load test showed fewer failed deliveries"), is allowed.

FAIL if the title or body states any of the load-test figures, the cost, the time estimate, the reverted first attempt, or the number of sessions.

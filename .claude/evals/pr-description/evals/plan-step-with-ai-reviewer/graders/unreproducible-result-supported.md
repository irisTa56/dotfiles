---
type: llm
---

The author's notes report a load test on staging (10,000 deliveries, failure rate 3.1 % to 0.4 %), an environment the reviewer cannot reach or rerun.
The notes give no premises about that environment beyond its name.
Judge only the title and body; ignore any note to the author after the body.

PASS if the body either leaves the load test out, or does not lean on it alone: the claim it would support (that retries recover transient failures) is also argued from how the change works or from how backoff is generally known to behave, or the body states what the result depends on before giving it.

FAIL if the body offers the load test, with its figures or as a bare verdict such as "a staging load test showed fewer failures", as the support for that claim with nothing a reviewer could follow.

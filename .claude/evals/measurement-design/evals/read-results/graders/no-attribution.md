---
type: llm
---

Runs A and B differ by build (v2.13 against v2.14, which may hold other changes besides the cache) and by date, so they cannot show what the cache alone did.

PASS if the response does not credit the cache with the difference as a measured fact: it either declines to, or attributes the difference to the v2.14 release as a whole and says that the cache's own share was not isolated.

FAIL if the response, in the note or outside it, states that the cache sped requests up by some amount (such as "the cache cut median latency by 17%") without saying that the comparison does not isolate the cache.

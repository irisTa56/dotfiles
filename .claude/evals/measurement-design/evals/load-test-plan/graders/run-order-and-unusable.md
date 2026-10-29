---
type: llm
---

PASS if the plan does both of these:

- runs the pool sizes interleaved within the session (alternating or shuffled per round) rather than as one block per pool size, with the order written out or fixed before the first run;
- states what makes a run unusable (for example the load generator saturating, errors, or a restart mid-run) and what happens to such a run (for example it is excluded and counted, or replaced within a stated limit).

FAIL if the plan runs each pool size as one block, leaves the order to be chosen during the run, or has no criteria for an unusable run, or lets unusable runs be replaced without limit.

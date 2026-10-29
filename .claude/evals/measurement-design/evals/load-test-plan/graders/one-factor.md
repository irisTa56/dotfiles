---
type: llm
---

The author's notes have the pool 40 run also switch the app to a new JSON serializer.

PASS if no condition in the plan is compared against another condition from which it differs in both pool size and serializer: the serializer change is dropped, or given a condition of its own.

FAIL if the plan compares pool 20 against a pool 40 run that also uses the new serializer.

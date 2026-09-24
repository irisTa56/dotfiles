---
type: llm
---

The rule covers new code only, and `legacy_io.py` is existing code this change does not touch. The finding claims a violation of a standard that does not say what it claims.

PASS if the reply does not convert `legacy_io.py` in this change and gives as a reason that the rule covers new code. Offering the conversion as a separate change, or leaving it to the author, still passes.

FAIL if the reply accepts converting `legacy_io.py` in this change, or declines only because the conversion is out of scope without noting that the rule does not require it.

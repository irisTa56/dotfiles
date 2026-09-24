---
type: llm
---

The author made three decisions not to change something: the CSV delimiter stays a comma, the deprecated `/v1/export` endpoint keeps the old behavior, and the function name `build_monthly_rows` stays.

Decide by where each of them is mentioned, not by how it is worded. Naming `build_monthly_rows` as the function that was fixed is not a mention of the decision to keep its name.

PASS if none of the three is mentioned, or each is mentioned only in a section whose heading marks it as outside the change (such as "Scope", "Out of scope", or "Not included").

FAIL if any of the three is mentioned anywhere inside a section that describes what the PR changes (such as "Fix", "Change", or "Summary"), whether as a bullet or a sentence.

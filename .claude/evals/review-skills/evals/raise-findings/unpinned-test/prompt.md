---
description: A bug fix whose regression test passes against the old code as well as the new
max_turns: 10
allowed_tools: [Skill]
---

Review the change below by running the raise-findings skill. There is no git repository here: the diff below is the change, and it shows every file it touches in full.

The change fixes a bug report: "Saturday shifts are billed at the weekday rate."

```diff
--- a/billing.py
+++ b/billing.py
@@ -1,9 +1,9 @@
 from datetime import date


 def is_weekend(day: date) -> bool:
-    return day.weekday() > 5
+    return day.weekday() >= 5


 def hourly_rate(day: date, base: float) -> float:
     return base * 1.5 if is_weekend(day) else base
--- a/tests/test_billing.py
+++ b/tests/test_billing.py
@@ -1,8 +1,12 @@
 from datetime import date

 from billing import hourly_rate


 def test_weekday_rate():
     assert hourly_rate(date(2024, 6, 5), 20.0) == 20.0
+
+
+def test_weekend_rate():
+    assert hourly_rate(date(2024, 6, 9), 20.0) == 30.0
```

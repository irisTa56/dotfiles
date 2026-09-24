---
description: A behavior-preserving refactor with nothing wrong in it, where a reviewer can only suggest growth or preferences
max_turns: 10
allowed_tools: [Skill]
---

Review the change below by running the raise-findings skill. There is no git repository here: the diff below is the change, and it shows every file it touches in full.

The change is a refactor: it names the request timeout that two calls repeated as a literal. No behavior is meant to change.

```diff
--- a/client.py
+++ b/client.py
@@ -1,15 +1,17 @@
 import requests

+REQUEST_TIMEOUT_SECONDS = 30
+

 def fetch_invoice(session: requests.Session, base_url: str, invoice_id: str) -> dict:
-    resp = session.get(f"{base_url}/invoices/{invoice_id}", timeout=30)
+    resp = session.get(f"{base_url}/invoices/{invoice_id}", timeout=REQUEST_TIMEOUT_SECONDS)
     resp.raise_for_status()
     return resp.json()


 def fetch_customer(session: requests.Session, base_url: str, customer_id: str) -> dict:
-    resp = session.get(f"{base_url}/customers/{customer_id}", timeout=30)
+    resp = session.get(f"{base_url}/customers/{customer_id}", timeout=REQUEST_TIMEOUT_SECONDS)
     resp.raise_for_status()
     return resp.json()
```

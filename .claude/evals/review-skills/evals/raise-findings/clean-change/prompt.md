---
description: A small correct change with tests, where only preferences are left to raise
max_turns: 12
allowed_tools: [Skill, Read, Grep, Glob]
---

Review the change below by running the raise-findings skill. There is no git repository here: the diff is the change, and the files after it are under `resources/`.

The change is for turning post titles into URL slugs in a blog generator. Titles are written by the blog's single author, and a slug only has to be readable; nothing depends on two titles getting different slugs.

```diff
--- /dev/null
+++ b/slug.py
@@ -0,0 +1,8 @@
+import re
+import unicodedata
+
+
+def slugify(s: str) -> str:
+    s = unicodedata.normalize("NFKD", s).encode("ascii", "ignore").decode("ascii")
+    s = re.sub(r"[^a-z0-9]+", "-", s.lower())
+    return s.strip("-")
--- /dev/null
+++ b/test_slug.py
@@ -0,0 +1,17 @@
+from slug import slugify
+
+
+def test_lowercases_and_joins_words():
+    assert slugify("Hello World") == "hello-world"
+
+
+def test_drops_accents():
+    assert slugify("Crème Brûlée") == "creme-brulee"
+
+
+def test_collapses_and_trims_separators():
+    assert slugify("  --a  b--  ") == "a-b"
+
+
+def test_empty_input():
+    assert slugify("") == ""
```

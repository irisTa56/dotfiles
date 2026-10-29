---
type: llm
---

Look at every `<...>` placeholder in the run sheet's command blocks.

PASS if each one appears only on a variable assignment at the head of a block (such as `CACHE_MB=<size>`), and the commands below it refer to the variable rather than to another `<...>` placeholder.
A run sheet with no `<...>` placeholder at all also passes.

FAIL if a `<...>` placeholder appears inside a command line other than such a variable assignment, for example `--out results/<size>.csv`.

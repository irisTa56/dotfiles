---
type: llm
---

The response is a pull request body for a dependency refresh that moved eight direct dependencies.

PASS if the body summarizes the version changes one level up (for example "bumped the direct dependencies and refreshed the lockfile") and names the from/to versions of at most two packages.

FAIL if it lists the from/to versions of three or more packages, in a list, a table, or prose.

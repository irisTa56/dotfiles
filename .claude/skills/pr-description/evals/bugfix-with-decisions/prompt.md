---
description: A bug fix whose notes mix real changes with things deliberately left alone and internal details nobody would question
max_turns: 8
allowed_tools: [Skill]
---

Write the pull request description for this change. There is no PR template; the team writes PR bodies in English. Reply with the PR body only.

My notes on the change:

- Bug: the monthly CSV export dated each row in the server's timezone (UTC), so customers west of UTC saw last-day-of-month orders under the next month.
- Fix: the export now converts each order's timestamp to the account's configured timezone before bucketing it into a month, and writes the date in that timezone.
- Added a regression test with an order at 23:30 on 31 January in America/Los_Angeles, which must land in January.
- I considered also switching the delimiter from comma to semicolon for European Excel users, but decided against it; that belongs in its own change.
- I did not touch the deprecated `/v1/export` endpoint; it keeps the old behavior until it is removed next quarter.
- The timezone conversion uses the `zoneinfo` module from the standard library, which we already import elsewhere.
- I kept the function's name `build_monthly_rows` unchanged.

---
description: Notes for a load-test plan whose traps are not flagged — last week's numbers on an older build, a capacity figure from the instance rating, an open-ended next step, an open-ended run count, and a bare time estimate
max_turns: 8
allowed_tools: [Skill]
---

We're deciding whether to raise our web service's database connection pool from 20 to 40 in production. Write me a plan for a load test on staging to settle it, in Markdown. Reply with the plan only.

Some context:

- Pool 20 numbers are already on the dashboard from last Thursday's load test: p95 180 ms at 2,000 rps.
- Staging was upgraded to v2.14 on Monday; that release ships the new JSON serializer.
- DB headroom is fine: the instance class is rated for 5,000 connections.
- Do 10 runs at pool 40, and add more if it looks noisy.
- After pool 40 we'll see whether 30 or 60 is worth trying.
- It should take about an hour.

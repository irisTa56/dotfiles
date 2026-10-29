---
description: Notes for a load-test plan that change two factors at once, reuse an old baseline, include a test-only knob, lean on a vendor figure, leave the next step to the first result, and mix iteration counts
max_turns: 8
allowed_tools: [Skill]
---

We're deciding whether to raise our web service's database connection pool from 20 to 40 in production, and I want a load test on the staging cluster to settle it. Write me the test plan in Markdown. Reply with the plan only.

My notes:

- Last week's load test at pool size 20 gave a p95 latency of 180 ms, so we can use that as the baseline and only run pool 40 this time.
- The pool 40 run will also switch the app to the new JSON serializer, since that is going out next month anyway.
- Also try pool 40 with the `--turbo` flag; it only exists in the test build, but it might look good.
- The database vendor's datasheet says the instance handles 5,000 connections, so the database side won't be the bottleneck.
- Run pool 40 for 37 iterations, and the turbo run for 50.
- We'll look at the first result and decide what to try next.
- The whole thing should take about an hour.

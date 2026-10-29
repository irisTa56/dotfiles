---
description: A change made as one step of a plan, with run results, estimates and progress history on offer, for a repository whose PRs an AI reviewer reads
max_turns: 8
allowed_tools: [Skill]
---

Write the pull request title and description for this change.
There is no PR template; the team writes PRs in English.
An automated AI reviewer reads every PR body with no other context, and the team asks each PR to give it a "Review focus" section.
Reply with the title on the first line, then the body.

My notes on the change:

## Step 2.3 (session 4): webhook retries

- Change: the webhook delivery worker now retries a failed delivery with exponential backoff instead of dropping it, up to `WEBHOOK_MAX_RETRIES` (default 5).
- Each retry that is given up on increments `webhook_delivery_failures_total`.
- Added unit tests for the backoff schedule and for the retry cap.
- Load test on staging: 10,000 deliveries, failure rate went from 3.1 % to 0.4 %.
- My first attempt used a fixed 2-second delay; I reverted it because it hammered slow receivers. This took two sessions.
- This session cost about $3 in model usage; step 2.4 (dead-letter queue) should take about one more day.
- What I want the reviewer to look at: whether the production failure rate will really drop below 1 %, and whether the backoff ever exceeds the cap.

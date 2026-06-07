---
name: review-loop
description: "Orchestrate an iterate-until-clean review of code you just changed: delegate the review to a subagent, judge each finding, then loop fix and re-review until no valid finding remains. Use after a non-trivial code change, or when the user asks to repeatedly review until clean. Delegation + loop + review mindset; the subagent does the actual review pass (e.g. via code-review-expert or /code-review)."
---

# Review Loop

## Workflow

1. **Review in a subagent.** Spawn a subagent to review the current changes (e.g. via the `code-review-expert` skill or `/code-review`), so the review comes from a clean, independent perspective (and keeps the main context uncluttered). It returns findings only — no edits.
2. **Report findings** to the user as the subagent returned them.
3. **Judge each finding's validity**; state which you accept or reject and why.
4. **Loop**: fix the valid findings per the mindset below, then spawn a fresh review subagent. Repeat until a review pass returns no valid findings.

## Mindset for Addressing Findings

Apply when judging findings and applying fixes:

1. Step back and view the findings holistically; aim for a root-cause fix, not a local patch.
2. Even if an issue looks minor now, address it when it has "broken-window" character or risks future rework.
3. Strictly avoid symptomatic fixes, tolerating the status quo with a comment, or rejecting a fix on the grounds that the existing code already does it this way.
4. Check related sites for missed fixes; keep consistency and coherence across them.
5. Don't follow the spec or plan blindly — reason from the actual behavior, and surface suspected spec/plan defects with the user.

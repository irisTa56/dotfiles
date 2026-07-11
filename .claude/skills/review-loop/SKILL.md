---
name: review-loop
description: "Orchestrate an iterate-until-clean review of code you just changed: delegate the review to a subagent, judge each finding, then loop fix and re-review until no valid finding remains. Use after a non-trivial code change, or when the user asks to repeatedly review until clean. Delegation + loop + review mindset; the subagent does the actual review pass via the code-review-expert skill."
---

# Review Loop

## Workflow

1. **Review in a subagent.** Spawn a general-purpose subagent (the `Agent` tool) and, in its prompt, instruct it to review the current changes by running the `code-review-expert` skill, emphasizing the perspectives below, so the review comes from a clean, independent vantage point (and keeps the main context uncluttered). `code-review-expert` is a Skill, not an agent type — do NOT pass it as `subagent_type` (that call fails). The subagent returns findings only — no edits.
2. **Report findings** to the user as the subagent returned them.
3. **Judge each finding's validity**; state which you accept or reject and why.
4. **Loop**: fix the valid findings per the mindset below, then spawn a fresh review subagent. Repeat until a review pass returns no valid findings.

## Perspectives for the Review

Direct the reviewer to weigh these on top of the `code-review-expert` defaults:

1. Whether the change is organically integrated into the deliverable as a whole, rather than a surface-level feature implementation — coherent with the existing design and optimized in context, not bolted on.
2. Whether the automated tests are sound in quality and coverage, and whether they exercise externally observable behavior rather than internal implementation details.
3. Whether declarative tooling config (e.g. `pyproject.toml`, `ruff.toml`, `mise.toml`, `.gitignore`) is authored minimally — only deviations from default, with no pre-emptive defense.
   - Flag added lines that:
     - restate a tool's default value, rather than configuring only what a concrete, already-encountered problem requires;
     - pre-emptively ignore lint rules or add "just in case" suppressions for problems that have not occurred;
     - defensively pin or bound pre-1.0 dependency versions absent an observed break;
     - embed process/progress notes (e.g. "committed once Phase N lands") in shipped config.
   - Treat these as maintainability findings (typically P2/P3), not correctness.

## Mindset for Addressing Findings

Apply when judging findings and applying fixes:

1. Step back and view the findings holistically; aim for a root-cause fix, not a local patch.
2. Even if an issue looks minor now, address it when it has "broken-window" character or risks future rework.
3. Strictly avoid symptomatic fixes, tolerating the status quo with a comment, or rejecting a fix on the grounds that the existing code already does it this way.
4. Check related sites for missed fixes; keep consistency and coherence across them.
5. Don't follow the spec or plan blindly — reason from the actual behavior, and surface suspected spec/plan defects with the user.

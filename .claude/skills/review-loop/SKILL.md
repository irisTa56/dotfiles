---
name: review-loop
description: "Orchestrate an iterate-until-clean review of code you just changed: delegate the review to a subagent, judge each finding, then loop fix and re-review until no valid finding remains. Invoke when the user asks to pass the current changes through review, or when a workflow step calls for a review pass. The subagent does the actual review via the code-review-expert skill and makes no edits."
---

# Review Loop

## Workflow

1. **Review in a subagent.** Spawn a general-purpose subagent (the `Agent` tool) and, in its prompt, instruct it to review the current changes by running the `code-review-expert` skill with the perspectives below — a clean, independent vantage point that also keeps the main context uncluttered.
   - `code-review-expert` is a Skill, not an agent type — do NOT pass it as `subagent_type` (that call fails).
   - The subagent returns findings only; it makes no edits.
2. **Report findings** to the user as the subagent returned them.
3. **Judge and fix with `address-finding`.** Apply the `address-finding` skill (invoke it via the Skill tool) to judge each finding's validity and fix the valid ones. State which you accept or reject and why.
4. **Loop.** Spawn a fresh review subagent and repeat until a pass returns no valid finding.
   - When the loop settles, take one holistic look that the accumulated fixes read as a coherent whole rather than a stack of independent patches. Coherence is the target — not diff size.

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

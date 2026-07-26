---
name: review-loop
description: "Orchestrate an iterate-until-clean review of code you just changed: delegate the review to a subagent, judge each finding, then loop fix and re-review until every valid finding is either fixed or settled with the user. Invoke when the user asks to pass the current changes through review, or when a workflow step calls for a review pass. The subagent does the actual review via the code-review-expert skill and makes no edits."
---

# Review Loop

## Workflow

1. **Establish the purpose.** State what this change is for before the first round, and hold every later round to that statement. It is what `address-finding` weighs a fix against, and letting each round re-infer it from a diff the last round grew turns the bound into a ratchet.
   - Take it from the user, or failing that from the workflow that invoked this loop; when neither states one, infer it from the diff and say so before the first round.
   - Do not wait for a reply: state it and continue, so a correction is available and costs nothing to skip.
   - This statement is `address-finding`'s required purpose statement for every round, so it is not re-announced at the start of one. Restate it when putting an excess to the user, who is judging against it.
   - Work that has landed is in-bound for later rounds: an excess the user chose to extend the change with, and a correction `address-finding` landed outside the bound on its own. Later rounds may fix defects in it, and neither licenses a further excursion past the purpose.
2. **Review in a subagent.** Spawn a general-purpose subagent (the `Agent` tool) and, in its prompt, instruct it to review the current changes by running the `code-review-expert` skill with the perspectives below — a clean, independent vantage point that also keeps the main context uncluttered.
   - `code-review-expert` is a Skill, not an agent type — do NOT pass it as `subagent_type` (that call fails).
   - The subagent returns findings only; it makes no edits.
3. **Report findings** to the user as the subagent returned them.
4. **Judge and fix with `address-finding`.** Apply the `address-finding` skill (invoke it via the Skill tool) to judge each finding's validity and fix the valid ones. State which you accept or reject and why.
   - When a fix reaches past the purpose bound, that skill puts the excess to the user.
     - This is the loop's one blocking pause: wait for the answer before continuing the round.
     - Step 1's "do not wait" governs the purpose statement alone.
   - Record what a round knowingly leaves undone, and what the user decided about it. Anything `address-finding` surfaced rather than fixed goes in the record whether or not the user answered.
   - Anything so recorded is **settled**, so a later round that reports it again is answered from the record rather than escalated afresh. A valid finding recorded this way stays valid.
   - For `address-finding`'s no-silent-reversal check, the piece of work is the change under review together with the fixes and the record this loop has accumulated — not the current round alone.
5. **Loop.** Spawn a fresh review subagent and repeat until a pass returns no valid finding that is not already settled.
   - A round that applied a fix cannot be the last one. That fix is unreviewed, and the loop exists because an unreviewed fix is where a serious defect hides — spawn again even when you expect nothing.
   - When the loop settles, take one holistic look that the accumulated fixes read as a coherent whole rather than a stack of independent patches. Coherence is the target — not diff size.
   - A fix that the holistic look calls for goes through `address-finding` like any finding, purpose bound included, and the loop re-enters at step 2 so the consolidation is itself reviewed.
   - When the loop closes, report the record, so what was left undone does not end with the loop.

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

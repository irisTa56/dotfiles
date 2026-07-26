---
name: review-loop
description: "Orchestrate an iterate-until-clean review of code you just changed: delegate the review to a subagent, judge each finding, then loop fix and re-review until every valid finding is either fixed or settled with the user. Invoke when the user asks to pass the current changes through review, or when a workflow step calls for a review pass. The subagent does the actual review via the code-review-expert skill and makes no edits."
---

# Review Loop

## Workflow

1. **Fix the purpose.** State what this change is for before the first round, and hold every later round to that statement. It is what `address-finding` weighs a fix against, and letting each round re-infer it from a diff the last round grew turns the bound into a ratchet.
   - Take it from the user. When they have not stated one, infer it from the diff and put that inference to them before the first round, while correcting it is still cheap.
   - An escalation the user accepts brings that work into the change, so later rounds may fix defects in it. It does not licence a further excursion past the purpose.
2. **Review in a subagent.** Spawn a general-purpose subagent (the `Agent` tool) and, in its prompt, instruct it to review the current changes by running the `code-review-expert` skill with the perspectives below — a clean, independent vantage point that also keeps the main context uncluttered.
   - `code-review-expert` is a Skill, not an agent type — do NOT pass it as `subagent_type` (that call fails).
   - The subagent returns findings only; it makes no edits.
   - Keep the ledger, the pinned purpose, and every earlier round's outcome out of the prompt. Not knowing them is what the reviewer is for.
3. **Report findings** to the user as the subagent returned them.
4. **Judge and fix with `address-finding`.** Apply the `address-finding` skill (invoke it via the Skill tool) to judge each finding's validity and fix the valid ones. State which you accept or reject and why.
   - **Keep a ledger.** Record every finding a round does not simply fix, with the decision taken and who took it.
     - This covers two classes:
       - an escalation the user answered, including one they told you to fix here;
       - anything `address-finding` surfaces to the user instead of fixing.
     - A fresh reviewer knows none of this and re-reports those findings.
       - Where the user decided, answer from the ledger.
       - Where you decided alone, a second independent report is new evidence, so judge it again.
5. **Loop.** Spawn a fresh review subagent and repeat until a pass returns no valid finding the ledger has not already settled.
   - Before spawning:
     - settle everything you put to the user;
     - land whatever work the user's answer to an escalation left in the diff.
   - When the loop settles:
     - take one holistic look that the accumulated fixes read as a coherent whole rather than a stack of independent patches — coherence is the target, not diff size;
     - report every ledgered finding with the decision recorded for it, flagging carved-out work as follow-up that is owed.

## Perspectives for the Review

Direct the reviewer to weigh these on top of the `code-review-expert` defaults.
Across all of them, hold reported ambiguity to a bar: name two readings and the different actions they lead to, because an ambiguity that cannot change what the reader does is not a finding. Without it, a careful reviewer generates them without end.

1. Whether the change is organically integrated into the deliverable as a whole, rather than a surface-level feature implementation — coherent with the existing design and optimized in context, not bolted on.
2. Whether the automated tests are sound in quality and coverage, and whether they exercise externally observable behavior rather than internal implementation details.
   - Where the change alters executable behavior and a suite exists, name as a finding any behavior it introduces or alters that the suite does not pin, whether you established that by reading or by running the code. Leave the test design to the implementation side.
3. Whether declarative tooling config (e.g. `pyproject.toml`, `ruff.toml`, `mise.toml`, `.gitignore`) is authored minimally — only deviations from default, with no pre-emptive defense.
   - Flag added lines that:
     - restate a tool's default value, rather than configuring only what a concrete, already-encountered problem requires;
     - pre-emptively ignore lint rules or add "just in case" suppressions for problems that have not occurred;
     - defensively pin or bound pre-1.0 dependency versions absent an observed break;
     - embed process/progress notes (e.g. "committed once Phase N lands") in shipped config.
   - Treat these as maintainability findings (typically P2/P3), not correctness.

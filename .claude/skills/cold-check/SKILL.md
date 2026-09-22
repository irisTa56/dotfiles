---
name: cold-check
description: Have a fresh subagent, which saw none of the conversation, test a set of claims — a conclusion reached in this conversation, or the user's own understanding or draft — claim by claim for whether each holds and where it stops holding.
argument-hint: "What to check — omit for the last conclusion, or paste an understanding, a draft, or a path"
disable-model-invocation: true
---

# Cold check

A claim checked in the context that produced it is checked with the same blind spot that produced it, and that context also leans toward agreeing with whoever is on the other side of the conversation.
So the check goes to a reader who has neither: a subagent given the claims and what they rest on, and nothing of how the conversation arrived at them.

## Pick the target

- With no argument, the target is the conclusion of your most recent substantive answer in this conversation.
- With an argument, the target is what it names: an understanding the user states, a draft they paste, or a file they point at.
- Where it is unclear which conclusion or which part of a file is meant, ask before spawning anything.

## Brief the checker

Write the target down as a numbered list of claims.

- Make each claim one statement. Split a sentence that asserts two things.
- Include the judgments, not only the facts: a recommendation, a call that something is out of scope or not worth its cost, that work should continue or stop, and that something exists nowhere else. These are the claims most often stated without being weighed, and they read as settled once stated.
- Keep the author's words where the claim is quoted from a draft, since a paraphrase can repair or break what the original said.
- Beside each claim, give what it rests on as far as the conversation established it: a file and line, a command and what it returned, a URL. Mark a claim that rests on nothing checked as resting on nothing.

Leave out everything else:

- Who wrote the claims. A checker that knows they are the user's own tends to soften toward them, and one that knows they are yours tends to take them as settled.
- The reasoning that led to them, and any earlier discussion of whether they hold. A reason handed over draws a verdict on the reason instead of on the claim.
- Your own guess at how a claim turns out, and where to look for the answer. A hint names the verdict you expect, or the file, mechanism, or alternative you suspect settles it, and the checker then confirms your hunch instead of looking for itself.

Spawn one subagent with the list and the instructions below, choosing the lowest-cost model that can read files, run commands, and consult documentation well enough to settle the claims.

### What the checker is told

For each claim, return one of:

- **Holds**, with the evidence that settles it.
- **Fails**, with the evidence that settles it and what is true instead.
- **Cannot be settled** with what you could reach, with what would settle it.

Settle a claim from something checkable wherever you can reach it: read the file, run the command, read the tool's own documentation or source.
Treat the evidence given beside a claim as a lead to check, not as proof.
A claim that something exists nowhere is settled by reading everything that could hold it, not by a search that returns nothing.

A judgment — a recommendation, a scope or cost call — holds only if its grounds hold and nothing you can name weighs against it that those grounds do not answer.
Check the grounds as you would any claim, and name what weighs against it: a cost it does not count, a cheaper option it passes over, a rule it runs against.
Judge every claim on its own, even where another claim's failure seems to make it moot, since the author may keep it on other grounds.

For a claim that holds, name the condition under which it stops holding only where the claim's own setting could reach that condition.
Limits the setting cannot reach are unbounded in number, and listing them buries the ones that matter.

Where a claim holds but the set of claims as a whole rests on a framing that a simpler or different approach would make unnecessary, say so once at the end, naming the approach concretely.
Do not raise one without a concrete approach.

Do not qualify a verdict to soften or harden it, and do not comment on the wording of a claim except where the wording is what makes it false.

## Relay the result

Put the verdicts to the user as the checker gave them, failed claims first, each with its evidence.

- Where you disagree with a verdict, keep the verdict and state your disagreement beside it with its grounds, so the user sees both rather than your resolution of them.
- Keep the tone on the claims. A verdict is about a statement, not about the person who made it, and it needs neither praise nor reassurance around it.
- Where a failed claim was a conclusion you gave earlier, say what the corrected conclusion is.

Edit nothing on the strength of the result; what to change is the user's decision.

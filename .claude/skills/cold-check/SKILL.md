---
name: cold-check
description: Have a fresh subagent, which saw none of the conversation, test a set of claims — a conclusion reached in this conversation, or the user's own understanding or draft — claim by claim for whether each holds and where it stops holding, and set its verdicts beside the ones the conversation's own context reaches.
argument-hint: "What to check — omit for the last conclusion, or paste an understanding, a draft, or a path"
disable-model-invocation: true
---

# Cold check

A claim checked in the context that produced it is checked with the same blind spot that produced it, and that context also leans toward agreeing with whoever is on the other side of the conversation.
So the check goes to a reader who has neither: a subagent given the claims and what they rest on, and nothing of how the conversation arrived at them.
The conversation still holds what that reader lacks, such as what the user meant and what was decided, so your own verdict goes beside the checker's rather than being replaced by it.

## Pick the target

- With no argument, the target is the conclusion of your most recent substantive answer in this conversation.
- With an argument, the target is what it names, such as an understanding the user states, a draft they paste, or a file they point at.
- Where it is unclear which conclusion or which part of a file is meant, ask before spawning anything.

## Brief the checker

Write the target down as a numbered list of claims.

- Make each claim one statement. Split a sentence that asserts two things.
- Include the judgments, not only the facts, such as a recommendation, where a fix belongs, a call that something is out of scope or not worth its cost, that work should continue or stop, and that something exists nowhere else. These are the claims most often stated without being weighed, and they read as settled once stated.
- Keep the author's words where the claim is quoted from a draft, since a paraphrase can repair or break what the original said.
- Beside each claim, give what it rests on as far as the conversation established it, such as a file and line, a command and what it returned, or a URL. Mark a claim that rests on nothing checked as resting on nothing.
- State what the user reported as having happened, and what they decided, as the setting rather than as claims. The checker cannot reach either, so as claims they come back as cannot be settled and bury the verdicts that matter.

Leave out everything else, including:

- Who wrote the claims. A checker that knows they are the user's own tends to soften toward them, and one that knows they are yours tends to take them as settled.
- The reasoning that led to them, and any earlier discussion of whether they hold. A reason handed over draws a verdict on the reason instead of on the claim.
- Your own guess at how a claim turns out, and where to look for the answer. A hint names the verdict you expect, or the file, mechanism, or alternative you suspect settles it, and the checker then confirms your hunch instead of looking for itself.

Before spawning, write down your own verdict on each claim, in the same three terms the checker uses, from everything the conversation holds and anything you check now.
Keep it out of the brief, and fix it before the checker's result arrives, so that result does not pull yours toward it.

Spawn one subagent with the list and the instructions below.
It runs commands on the user's machine, so choose the lowest-cost model that both settles the claims well and keeps to the read-only limit reliably, which means no smaller than Sonnet.

### What the checker is told

For each claim, return one of:

- **Holds**, with the evidence that settles it.
- **Fails**, with the evidence that settles it and what is true instead.
- **Cannot be settled** with what you could reach, with what would settle it.

Settle a claim from something checkable wherever you can reach it, for example by reading the file, running the command, or reading the tool's own documentation or source.
Treat the evidence given beside a claim as a lead to check, not as proof.
Change nothing while you check: run only what reads, and never anything that writes, wherever it writes — a file anywhere on the machine, git state, or an external system, such as a push, a sync that overwrites local files, a gist edit, or an API call that modifies.
The one exception is a temporary directory you create for this check, where you may write what reading needs, such as a clone to inspect; nothing run there may reach outside it.
Where only a write could settle a claim, the claim cannot be settled; name the write that would settle it.
A claim that something exists nowhere is settled by reading everything that could hold it, not by a search that returns nothing.

A judgment, such as a recommendation or a scope or cost call, holds only if its grounds hold and nothing you can name weighs against it that those grounds do not answer.
Check the grounds as you would any claim, and name whatever weighs against it, such as a cost it does not count, a cheaper option it passes over, or a rule it runs against.
Judge every claim on its own, even where another claim's failure seems to make it moot, since the author may keep it on other grounds.

For a claim that holds, name the condition under which it stops holding only where the claim's own setting could reach that condition.
Limits the setting cannot reach are unbounded in number, and listing them buries the ones that matter.

Where a claim holds but the set of claims as a whole rests on a framing that a simpler or different approach would make unnecessary, say so once at the end, naming the approach concretely.
Do not raise one without a concrete approach.

Do not qualify a verdict to soften or harden it, and do not comment on the wording of a claim except where the wording is what makes it false.

## Relay the result

Put the verdicts to the user claim by claim, with the checker's verdict as it gave it beside the one you wrote down: lead with the claims where the two conflict, then every other claim either judges failed, and let the rest follow.
Two verdicts conflict where one holds and the other fails, or where the checker settles what you could not; a claim the checker could not settle because only the conversation holds the answer is not a conflict.

- Where the two agree, give the verdict once with its evidence.
- Where they conflict, give both with their grounds and leave them unresolved. The conflict is what the user most needs to look at, and your resolution of it would be the same context judging itself again.
- Where the checker could not settle a claim because only the conversation holds the answer, say so, and let your verdict stand as the context's view.
- Keep the tone on the claims. A verdict is about a statement, not about the person who made it, and it needs neither praise nor reassurance around it.
- Where either verdict judges failed a claim that was a conclusion you gave earlier, say what the conclusion becomes if that verdict stands.

Edit nothing on the strength of the result; what to change is the user's decision.

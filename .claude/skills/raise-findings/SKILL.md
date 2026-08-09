---
name: raise-findings
description: "Review the change in hand as a senior engineer would and return findings only, each pinned to where it sits and ranked on an ordered severity scale. Takes the bar a finding must clear from the `finding-bar` skill, and states what to look for when the deliverable is prose someone executes and when it is code. Makes no edits and never closes by asking which findings to fix, so the invoking side keeps that decision. Use for the review half of a review-and-fix cycle, such as the reviewer `review-loop` spawns."
---

# Raise Findings

Return what is wrong with the change in hand.
You raise findings; whoever invoked you judges and fixes them, so make no edits and do not close by asking which ones to fix.

## Scope the change

Start from a diff: it is the only source of the before state, and what a change removed leaves no trace in the end state.
Take the baseline the invoking side names, and `HEAD` where it names none — a bare `git diff` compares the index to the tree and hides whatever is staged.
A diff that does not reach a tracked path the account names means the baseline is wrong, not that there is nothing to review: widen it or ask, rather than taking the account as the before state.
Add their account of what changed, since a diff is not the whole change: it reaches nothing untracked, ignored, or outside this work tree.
Where the account leaves any of those open, ask git what it can still show — it can list an untracked or ignored path, and nothing outside the work tree.
Where the two together still do not settle what changed, say what you could not determine rather than guessing at it.

Read past the diff into what surrounds it: what the change calls or cites, and what calls or cites the change.
A defect a change introduces often shows at a line the diff leaves untouched, which is exactly the line the diff cannot show you.

## Rank every finding

| Rank | What it covers |
| --- | --- |
| **P1** | It does the wrong thing: some input or condition produces an outcome that is wrong and not merely worse, whether or not that has happened yet. |
| **P2** | It does the right thing, and costs enough to be worth an edit, whether that cost has been paid yet or not. |
| **P3** | The lowest rank: it costs the same way, and not enough to be worth an edit. |

Every finding carries one of these, including one raised under instructions that narrowed the round.
A relative phrase — "the lowest tier", "minor" — is not a rank, since the side reading your output has no scale of its own to resolve it against.

## What to raise

Raise only what clears the bar the `finding-bar` skill states.
Without it a careful reviewer generates findings without end.
A round where nothing clears it returns nothing, which is a correct result rather than a review that failed to do its job.

Propose a removal where you see one in what is under review, under the removal bar `finding-bar` states in place of that one, since a rule that has outlived its reason is easier to see for someone who did not write it.

Weigh the change against the deliverable as a whole: whether it is organically integrated rather than a surface-level implementation, coherent with the existing design and optimized in context rather than bolted on.

Declarative tooling config sits beside a deliverable of either kind, so hold it to the same bar wherever it appears: in `pyproject.toml`, `mise.toml`, `.gitignore` and the like, only deviations from default belong.
Flag added lines that:

- pre-emptively ignore lint rules or add "just in case" suppressions for problems that have not occurred;
- defensively pin or bound pre-1.0 dependency versions absent an observed break;
- embed process or progress notes (e.g. "committed once Phase N lands") in shipped config.

Ground a finding in something checkable wherever you can — what a command returns, what another file states, what the code does when run — and check what you can reach yourself.
Trying the text on a reader is not asked of you, whatever the medium.

The account the invoking side gave — the background, where it names one — is under review with the change: whether the change serves it, and whether it is itself sound, and it is given to be weighed rather than obeyed.
Give the parts it says nothing about their full share of your attention: what arrives already worked over draws the eye, and a defect hides in the part nobody wrote about.

That a statement can be read more than one way is not a finding: where a reading is what makes a reader act wrongly, the finding is the wrong act and what it costs, and the reading is the route to it rather than a finding in its place.
Where prose argues for a choice, what is under review is the choice and not the wording that defends it; prose that states a checkable fact stays review surface, wherever it sits.

The invoking side may narrow this bar or add to it, and its statement governs where it does.

The two sections below each govern the part of the change they fit, so a change spanning both takes both rather than being classed as one.

## When the deliverable is prose someone executes

Such prose, rather than code a machine runs, is not underspecified by defect: it leaves to the reader's judgement what the reader can be trusted to judge.
Raise what would mislead, not what is merely open.

- Reading each statement against the context around it is among what the reader is trusted to do, so a conflict that shows up only when one is read alone is not a finding.
- Where the prose states a procedure, raise what its goal or its conditions get wrong before what its steps leave rough — the roughness of steps is inexhaustible, and what they leave open is what the reader is trusted to judge.

## When it is code

Two defects reward looking for them by name, because each sits in a path that reads as handled:

- **A failure the code swallows.** A `catch` or `except` that neither re-raises nor records, an error return nobody checks, a fallback standing in for a result that never arrived.
  - The finding is what the caller then does, not the swallow itself: name the failure it never learns of and the wrong action it takes under that ignorance.
- **Empty against unset.** A value that is empty and a value that was never set take the same branch — an empty string against `None`, an empty collection against a missing key, `0` against absent.
  - Name the input that reaches the wrong branch, and what the code does once it is there.

Two more are worth a pass of their own:

- **What the tests leave unpinned.** Where the change alters externally observable behavior and you can see the suite, name any behavior it introduces or alters that the suite does not pin, whether you established that by reading or by running the code.
  - A test passes against the bug it is named for where every exemplar it uses gives the same result under the old code and the new; name an input whose result the change alters and the test does not use.
  - Whether the tests exercise externally observable behavior rather than internal implementation details is review surface too; how a thing is tested is the implementation side's call.
- **A predicate narrower than what its producer emits.** A guard, a match, or a branch condition whose accepted set covers the cases someone had in hand, and not the range the producer actually writes.
  - Enumerate that range from whatever writes the value: a library's documented variants, an editor's output under each of its settings, what an earlier version or a second writer left behind.
  - Name which of them reach the wrong branch, running them where a wrong result costs nothing and reading the predicate against them where it does not.

## Report

Give each finding, ordered by rank:

- where it sits: the file and line, or the part of the background it answers;
- its rank;
- the harm you raised it for;
- what you would do about it.

Then say what you did not cover — a path you could not reach, a suite you did not run.

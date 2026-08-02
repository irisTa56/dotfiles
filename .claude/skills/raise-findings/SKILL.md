---
name: raise-findings
description: "Review the change in hand as a senior engineer would and return findings only, each pinned to where it sits and ranked on an ordered severity scale. Makes no edits and never closes by asking which findings to fix, so the invoking side keeps that decision. Use for the review half of a review-and-fix cycle, such as the reviewer `review-loop` spawns."
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
| **P1** | It does the wrong thing: some input, condition, or reading of it produces an outcome that is wrong and not merely worse, whether or not that has happened yet. |
| **P2** | It does the right thing, and costs enough to be worth an edit, whether that cost has been paid yet or not. |
| **P3** | The lowest rank: it costs the same way, and not enough to be worth an edit. |

Every finding carries one of these, including one raised under instructions that narrowed the round.
A relative phrase — "the lowest tier", "minor" — is not a rank, since the side reading your output has no scale of its own to resolve it against.

## What to raise

The bar a finding must clear is the invoking side's to state, and `review-loop` states it in its `## Perspectives for the Review`, copied whole into your prompt.
Read that section in the `review-loop` skill where you were given no such bar.

Two defects reward looking for them by name, because each sits in a path that reads as handled:

- **A failure the code swallows.** A `catch` or `except` that neither re-raises nor records, an error return nobody checks, a fallback standing in for a result that never arrived.
  - The finding is what the caller then does, not the swallow itself: name the failure it never learns of and the wrong action it takes under that ignorance.
- **Empty against unset.** A value that is empty and a value that was never set take the same branch — an empty string against `None`, an empty collection against a missing key, `0` against absent.
  - Name the input that reaches the wrong branch, and what the code does once it is there.

## Report

Give each finding, ordered by rank:

- where it sits: the file and line, or the part of the background it answers;
- its rank;
- the harm you raised it for;
- what you would do about it.

Then say what you did not cover — a path you could not reach, a suite you did not run — so the reader can tell an area you found clean from one you never examined.

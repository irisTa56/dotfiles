---
name: runbook
description: Write a procedure that a person executes from the page — a runbook, a setup or migration guide, an operating procedure, a run sheet for an experiment — or hand the user commands to run on their machine or a remote host. Use whenever drafting, restructuring, or finalizing such a document, and whenever a reply gives the user a sequence of commands to run and report back on, even when nobody says "runbook". Not for a script that runs unattended.
---

# Writing a procedure someone else executes

The executor has the page and the machine, and none of the conversation that produced the page.
Every gap your context fills while you read it back is a gap they meet mid-run.

Such a procedure is a [how-to guide](https://diataxis.fr/how-to-guides/) in Diátaxis' sense: it serves someone competent who has a task to get done, so it gives the actions and the judgement they need, and leaves teaching and explanation to other documents it can link.

Write its prose by [document-writing.md](../../rules/document-writing.md), and when it is in Japanese, compose it with the `japanese-tech-writing` skill as well.

## Decide before writing

- Settle a choice by measuring or by choosing, then write the outcome.
  - Hedging on the page — "this value is borrowed", "not verified for the other cases" — is the residue of a decision left unmade, and leaves the executor to make it mid-run.
  - Keep the deliberation itself off the page, but give a short reason wherever the executor would otherwise stop to wonder why a step or a value is what it is.
- Write a procedure, not a log of the run that produced it: the page reads as steps to take rather than as an account of what happened.
  - A figure from a past run can still help as what to expect, such as how long a step takes.
- Write no procedure for a situation not yet seen on the machine.
  - An unexercised procedure reads like a tested one; keep the thinking in the record and write the steps when the situation shows up.
- Where the procedure runs an experiment, the page carries every part of its design the executor acts on or records, as the `measurement-design` skill settles it beforehand, such as what is varied and why, how each outcome reads, and what invalidates the comparison.
  - Send incidental explanation — how a machine step is implemented, a tool's behaviour, a defence of the method — to a code comment or the plan.

## Structure the page

- Order the text as it is executed, and never patch order with a cross-reference.
- Refer to another section only where its steps apply unchanged.
  - Where the executor would have to run part of it differently, the sections are cut wrong: restructure them rather than annotate the reference.
- Keep the paragraph that runs a command apart from the one that says what to rerun when it fails, and a check apart from the operation it confirms.
- A branch whose outcomes are the same is not a branch: write the one outcome, and only what the executor records or does differently.
- Across chapters, name the thing (the cache size, the load level) rather than relying on a generic word (the setting, the condition) to carry it.
- Do not explain what the output already shows.
- Add no rule that refers to something the document never defines, such as a time budget nobody set.

## Code blocks the executor runs

These hold for commands in a document and for commands you hand the user in a reply.

- Label each code block with where it runs: the local machine, or which host.
- Avoid making the executor compose or edit a command by hand: give every command complete, with its paths and values filled in or drawn from variables.
  - A value only the executor knows goes on a variable line at the head of the code block, as a `<...>` placeholder; the commands below refer to the variable, and none has a `<...>` to replace mid-line.
  - Give a variable used across code blocks a full, descriptive name (`cache_size_mb`, not `sz`), and set it again in a code block that runs in another shell or on another host.
- Never have a person verify or retype what the program already knows: the program compares, and fails when the comparison fails.
  - Chain the command that depends on a check on it with `&&`, whether the check is inline or a script that exits non-zero. A pasted code block runs on past a failed line, so without the chain the next command runs on bad data.
  - Have the check say what it compared when it fails, or the executor cannot tell a failed check from a skipped step.
- Keep a hand-run code block free of logic — no nested `if`, no choosing between outcomes. Move the logic into a script, or split the code block into stages.
- Let every command stream its output: no `> /dev/null`, `| tail` or `| grep` on a run.
  - The executor then sees progress and the failing line as they happen; read the key lines from the run's log afterwards.
- Give a command whose result depends on which shell interprets it, such as a redirect or an expansion in a command run through `ssh`, as an interactive step on the target host: log in, then run it or edit the file there.
  - As a one-liner, the shell that runs `ssh` applies whatever the quoting leaves outside, so a write can land in a file on the local machine instead.
- In a reply, hand the commands over in one message, in execution order.

## Read it back before handing over

- Read it as the executor would:
  - every step's object named;
  - every place that says a step repeats an earlier one with a change, checked against the code block below it;
  - everything started also stopped;
  - every count still true under each branch;
  - every demonstrative resolvable on the page.
- Before handing over a procedure document you wrote or restructured, also have a cold reader read the whole document with nothing else, since your own read-back shares the context that fills the gaps a reader without it meets.
  - The cold reader is a general-purpose subagent at `effort: high` that has seen none of the conversation and is told to make no edits.
  - This is a reader's pass, not a `raise-findings` review: it returns each of these it finds, with where it sits.
    - a term, flag, column or number used before it is defined, or without its base (a fraction or multiple of what);
    - a fact true only of this environment, stated as general;
    - a sentence restating what the output shows;
    - a reason that does not lead to a step;
    - a table that contradicts the text introducing it;
    - a demonstrative with no referent on the page.
- Commands handed over in a reply get your own read-back alone.
- Fix what it returns, then read the whole procedure against its purpose again, not only the lines changed.

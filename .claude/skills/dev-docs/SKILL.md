---
name: dev-docs
description: Plan and steer multi-session development through a small set of committed documents — a roadmap, an architecture overview, decision records (ADRs), and one plan per phase — instead of specs or step-by-step implementation plans. Use when the user asks to plan, start, re-plan, or close a phase or milestone; to write or revise a roadmap or ARCHITECTURE.md; to record a decision with the alternatives it rejected; or to implement work from a phase plan. Also use when work in a repository that keeps these documents produces a decision worth recording or changes its structure, and when deciding whether a task needs a plan at all.
argument-hint: "What to do — e.g. start phase 2, close phase 1, record a decision, implement dev/plan/phase-02-<slug>.md"
---

# Development documents

These documents let a person steer work that spans sessions without writing a specification of the current behavior.
The flow is intent, then implementation.
A specification is written only for a boundary that others have come to depend on, and only for that boundary.
What keeps current behavior from drifting is the test suite, not a document.
Each document holds one kind of information, chosen by how long that information stays true.

- **Roadmap**: requirements at the granularity of ideas, scope, what is out of scope, and the order of phases.
  - It changes when direction changes.
- **Architecture overview**: the current structure, its invariants, a short list of behavior visible from outside, and links to the decisions behind them.
  - It changes when the structure does.
- **Decision record (ADR)**: one decision, the findings it rests on, the alternatives it rejected and why, and what it costs.
  - It never changes except for its status, as when it is superseded.
- **Phase plan**: what one phase must achieve and how anyone will know it did.
  - A person steers the phase with it, and once the phase closes, it is a record.
- **Not written**: a specification of current behavior, and a committed step-by-step implementation plan.
  - When the steps need planning, plan mode produces a throwaway plan (see [Working in a phase](#working-in-a-phase)).

Before any pull request opens, run `review-loop` on its diff yourself.
Where the diff adds or changes these documents, the person also reads it, since the documents are where they steer.

## Fit the repository first

Before writing anything, read what the repository already says: `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING`, and any document of the kinds above it already keeps, with its template.
Where the repository has a convention, follow it and map the kinds above onto it: each document goes where the repository keeps its kind, in that format, adding only the elements the format lacks and the kind needs.
Where it has none and the user owns the repository, use the default layout in [references/layout.md](references/layout.md).
Where it covers some kinds and not others, place the rest as the default layout would, beside the documents the repository already keeps.

A repository that has not adopted these documents does not get them unasked.
If there is no roadmap and the user has not asked to start one, apply only what fits the task in hand, such as the plan-mode guidance below or an ADR in the repository's own format, and propose the rest rather than creating it.

Read the reference for each document you write or change, and only those:

- [references/roadmap.md](references/roadmap.md)
- [references/architecture.md](references/architecture.md)
- [references/adr.md](references/adr.md)
- [references/phase-plan.md](references/phase-plan.md)

## Starting a phase

1. Read the roadmap and the architecture overview, and find the phase in the roadmap.
   - If the work is not there, see [Work outside the roadmap](#work-outside-the-roadmap).
2. Research whatever could swing the direction, such as which data source or dependency to use, before writing the plan.
   - Start from the findings the research notes still hold from earlier research, such as the roadmap's.
   - Where the notes go is in [Research](#research).
3. Write the phase plan, and record each decision it makes by whether someone changing that part later would need the reason.
   - A decision that is costly to reverse, or whose reason the code does not show, gets an ADR.
   - Any other decision stays in the plan as one line, with the finding it rests on beside it, even when it took research.
4. If the research contradicts the roadmap, correct the roadmap in the same commit rather than leaving the plan to disagree with it.
5. Resolve every question whose answer could change the plan before committing it, so that the plan's Open questions hold only what can wait.
6. Commit the plan, its ADRs, and the roadmap's update for review.

The plan lands on the main branch in a pull request of its own, before the implementation, because later sessions start from the main branch and must be able to read it.
New ADRs land as Proposed.

## Working in a phase

Treat the phase plan as the instruction: implement toward its Done when items and verify with the checks they name.
Write each check that can be automated as a test alongside the code it verifies, end to end where the item describes behavior seen from outside.
The test is how the item is verified now, and it keeps guarding the behavior once the plan is closed and no longer updated.
Where a test-driven development skill is available, work that red-green loop through it.

The plan has already settled requirements, assumptions, decisions, and what counts as done, so none of those is by itself a reason to enter plan mode.
Enter plan mode only when how to build it is still open:

- several ways to build it remain within the decisions,
- the change reorganizes existing structure or spans many files,
- or the code is unfamiliar.

A plan-mode plan covers only how to build.
If it would change a requirement, an assumption, a decision, a dependency, or a Done when item, or it contains a decision the person should review with care, move that into the phase plan or an ADR as [Changing direction](#changing-direction) says, and get it reviewed before implementing on top of it.
The approval screen is the right place to review how, but it is not a record, so it is the wrong place to settle what.

### When to go back to the plan

Go back when the plan itself stops holding, which its own sections tell you:

- an assumption turns out false, including when a risk's warning sign appears,
- a requirement, or the check a Done when item names, cannot hold as written,
- or a decision proves unworkable, or the work needs a dependency the plan does not list.

Difficulty that stays within the plan's latitude is not a reason to go back, so solve it there.
When one of these happens, stop implementing, tell the person which assumption, requirement, or decision broke and what you observed, and follow [Changing direction](#changing-direction).
If you notice it in plan mode, say so rather than presenting a plan that routes around it.

## Changing direction

Revise the phase plan and commit the revision, so the diff shows what changed.
If the direction itself changes, research again and write a new ADR that supersedes the old one.
The old one's status line is the only part of it that changes.

Where the revision goes depends on whether implementation has started:

- **Before it starts, or when the implementation so far is set aside to start over**: the revision lands on the main branch in a pull request of its own, as the plan did.
- **Once it has started**: the revision is a commit of its own on the implementation branch, which the person reads before any code builds on it.

An ADR written mid-phase lands as Proposed, like one written at the start, and is settled when the phase closes.

## Closing a phase

1. Under each Done when item, add the evidence that it holds: the test run or command and its output, a screenshot, a link to a run.
2. For each item that should go on holding but has no test, add a line to the architecture overview's outside-visible behavior, since the closed plan will not be updated.
3. Update the architecture overview for what the phase changed.
   - For the first phase, this is usually where it is first written.
4. Settle each of the phase's ADRs: Accepted, Rejected, or superseded.
5. Mark the phase done in the roadmap and the plan closed, and do not edit the plan after this.

These changes go in the pull request that completes the last Done when item, or in one of their own if the implementation has already merged.

## Research

Research happens at three points:

- while writing or revising the roadmap, for what the project should cover and which way it could go,
- while writing a phase plan, for questions that could change direction,
- and during a work session, for how to implement, in plan mode if that helps.

Keep it in a working copy where you can commit, since some session recorders capture a conversation only when a commit follows it.

Keep the process apart from what it found:

- **The process**: stays with the branch, not the main branch.
- **A finding**: goes beside the decision or assumption it supports, with its source and the date you checked it.
  - Where the decision has an ADR, that is the ADR's context, and otherwise it is the phase plan.
  - Which decisions get an ADR is settled in [Starting a phase](#starting-a-phase), and for a choice the roadmap makes in [references/roadmap.md](references/roadmap.md), not by the finding.

Research notes are working files and never reach the main branch.
Where they go depends on whether they are fit to publish wherever the branch is pushed:

- **Notes that are**: a `research/` directory next to these documents, committed to the branch.
  - Remove them before the review, once their conclusions are in the ADR or the plan, so the review and the merged diff see only those.
  - The pull request still keeps every commit reachable after its branch is deleted, and on a public host those commits are public, which is why no other notes go here.
- **Any other notes**: a directory outside the working tree, the one `git config --get --type=path dev-docs.private-notes` prints, in a subdirectory named after the branch.
  - No commit to the repository can pick them up there, and removing a worktree does not delete them.
  - If the command prints nothing, ask the person for the directory and record it with `git config dev-docs.private-notes <path>`, rather than leaving such notes untracked in the repository.
  - Where the directory is a repository, commit the notes there.
  - Where the pull request should carry how a decision was reached, rewrite that part into `research/` in a form fit to publish.

A finding whose decision or assumption is not written yet, such as one from the roadmap's research that bears on a phase not yet planned, has nothing to sit beside, so it is kept in that directory outside the working tree, fit to publish or not, where it will still be when that phase is planned.
When writing a phase plan, copy into it and its ADRs the findings its decisions and assumptions rest on.
Check each finding again as you copy it, and give it the date of that check, since facts such as a service's load or a repository's activity can change within days.

If the repository has an `.entire/` directory, read [references/entire.md](references/entire.md) before researching.

## Work outside the roadmap

Track it where the repository's instructions say, and without such an instruction, in the repository's issues.
Whether work that should stay private goes to a private tracker is the repository's choice.

- **A fix whose change one sentence can describe**: no document.
  - Record it in the tracker if it needs tracking, and close it with the pull request.
- **Work that one sentence cannot describe but that needs no steering**, such as a bug that needs investigation, a feature, or a refactor: the change goes in a pull request, with no phase plan.
  - For a bug, the report and the investigation go in the tracker.
  - A decision that comes out of it is recorded as in [Starting a phase](#starting-a-phase), phase or not.
  - A change to structure or invariants updates the architecture overview.
- **Work that needs steering**, meaning its requirements must be decided and it spans sessions: insert it into the roadmap as a phase and write a phase plan.

## Specifications

Do not write one by default.
When a boundary gains dependents outside it, such as a file format, an API, or a command-line interface others build on, propose a specification of that boundary alone, agree its location with the person, and link it from the architecture overview.

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

- Give the loop, as the use the change is built for, who reads each of those documents and what it leaves to another document or to the building session, as the list above and its reference state them.
  - A reviewer who is not told raises whatever the building session might trip on, and since each such finding is true, taking findings on their truth turns a plan into an implementation plan and a decision record into a list of a tool's behaviours.
- Where the diff adds an ADR, offer the person a `cold-check` of its claims before the pull request opens, since a claim found false after the merge can only be superseded.

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
   - Start from what earlier research, such as the roadmap's, left in the private workspace, and check a finding again before the plan rests on it, since facts such as a service's load or a repository's activity can change within days.
   - For a choice among candidates, take the candidates from a source that lists the field, such as a comparison page its community keeps, before turning any down, since a search that starts from the ones you know returns those.
   - Where settling an assumption takes something only the person can allow, such as a large download or an install, ask for it now rather than carrying the assumption into the plan unverified.
   - Where the notes and a download go is in [Research](#research).
3. Take the person through each decision that sets what the phase delivers or what it is built on before writing the plan, one decision at a time, with the candidates, what the research found for each, and the one you would pick.
   - A draft written on your own picks, with the questions beside it, has the person answer inside a direction they have not examined, and a review started on it reviews what their answers then overturn.
   - The reason recorded for a decision is one the person gave or a finding shows, so ask for a reason you would otherwise have to supply.
4. Write the phase plan, and record each decision it makes by whether someone changing that part later would need the reason.
   - A decision that is costly to reverse, or whose reason the code does not show, gets an ADR.
   - Any other decision the person steers by stays in the plan as one line, with the finding it rests on beside it, even when it took research.
   - A choice of how to build is neither, as [references/phase-plan.md](references/phase-plan.md) says of the plan's lines.
   - The plan and its ADRs say no more of another phase than the roadmap does, such as which phase comes next or what a later one will take up.
5. If the research contradicts the roadmap, correct the roadmap in the same commit rather than leaving the plan to disagree with it.
6. Resolve every question whose answer could change the plan before committing it, so that the plan's Open questions hold only what can wait.
7. Commit the plan, its ADRs, and the roadmap's update for review.

The plan lands on the main branch in a pull request of its own, before the implementation, because later sessions start from the main branch and must be able to read it.
New ADRs land as Proposed.

## Working in a phase

Treat the phase plan as the instruction: implement toward its Done when items and verify with the checks they name.
Start from the notes the planning session left for this plan in the private workspace, under that session's branch and not yours ([Research](#research)), and check a finding there again before code rests on it.
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
If the direction itself changes, research again and settle the new direction with the person as [Starting a phase](#starting-a-phase) does, then write a new ADR that supersedes the old one.
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

Run it from a working copy of the repository where you can commit, since some session recorders capture a conversation only when a commit to the repository follows it.

Keep the process apart from what it found, and a finding a document rests on apart from one only the building session needs:

- **The process**: its working files, such as notes, code written only to try something out, and downloaded data, stay out of the repository, in the private workspace that [references/private-workspace.md](references/private-workspace.md) describes.
  - No commit to the repository can pick them up there, and removing a worktree does not delete them.
- **A finding a decision or an assumption rests on**: goes beside it, with its source and the date you checked it.
  - A finding claims no more than was examined: where it speaks of a set, such as every file of a dataset or every library for a job, it names the members read, as in "the two files read hold no such column", and says "none", "every" or a range only where the whole set was read.
  - Where the decision has an ADR, that is the ADR's context, and otherwise it is the phase plan.
  - Which decisions get an ADR is settled in [Starting a phase](#starting-a-phase), and for a choice the roadmap makes in [references/roadmap.md](references/roadmap.md), not by the finding.
- **A finding only the building session needs**, such as how a tool behaved in a case you ran or which cases a test should hold: goes in the private workspace, in notes for that session that name the plan they are for.
  - It supports no decision, so no document has a place for it, and left among the working files it is found a second time while building.
  - Detail that leaves a plan or an ADR as how to build moves into those notes in the same step, so that nothing is dropped between the two.

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

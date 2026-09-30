---
name: dev-docs
description: Plan and steer multi-session development through a small set of committed documents — a roadmap, an architecture overview, decision records (ADRs), and one plan per phase — instead of specs or step-by-step implementation plans. Use when the user asks to plan, start, re-plan, or close a phase or milestone; to write or revise a roadmap or ARCHITECTURE.md; to record a decision with the alternatives it rejected; or to implement work from a phase plan. Also use when work in a repository that keeps these documents produces a decision worth recording or changes its structure, and when deciding whether a task needs a plan at all.
argument-hint: "What to do — e.g. start phase 2, close phase 1, record a decision, implement dev/plan/phase-02-<slug>.md"
---

# Development documents

These documents let a person steer work that spans sessions without writing a specification of the current behavior.
The flow is intent, then implementation; a specification is written only for a boundary that others have come to depend on, and only for that boundary.
Each document holds one kind of information, chosen by how long that information stays true.

- **Roadmap**: requirements at the granularity of ideas, scope, what is out of scope, and the order of phases. It changes when direction changes.
- **Architecture overview**: the current structure, its invariants, a short list of behavior visible from outside, and links to the decisions behind them. It changes when the structure does.
- **Decision record (ADR)**: one decision, the findings it rests on, the alternatives it rejected and why, and what it costs. It never changes except to be superseded.
- **Phase plan**: what one phase must achieve and how anyone will know it did. A person steers the phase with it; once the phase closes, it is a record.
- **Not written**: a specification of current behavior, and a committed step-by-step implementation plan. When the steps need planning, plan mode produces a throwaway plan (see [Working in a phase](#working-in-a-phase)).

Before any pull request opens, run `review-loop` on its diff yourself; where the diff adds or changes these documents, the person also reads it, since the documents are where they steer.

## Fit the repository first

Before writing anything, read what the repository already says: `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING`, an existing roadmap, and an existing decision-record directory with its template (`docs/adr/`, `doc/adr/`, `docs/decisions/`).
Where the repository has a convention, follow it and map the kinds above onto it: an ADR goes where the repository keeps them, in its template, adding only what the template lacks, such as rejected alternatives.
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

1. Read the roadmap and the architecture overview, and find the phase in the roadmap. If the work is not there, see [Work outside the roadmap](#work-outside-the-roadmap).
2. Research whatever could swing the direction, such as which data source or dependency to use, before writing the plan. Where the notes go is in [Research](#research).
3. Write the phase plan, and an ADR for each decision that later work must respect or will ask the reason for. A decision that matters only inside this phase stays in the plan as one line, with the finding it rests on beside it, even when it took research.
   If the research contradicts the roadmap, correct the roadmap in the same commit rather than leaving the plan to disagree with it.
4. Resolve every question whose answer could change the plan before committing it; the plan's Open questions hold only what can wait.
5. Commit the plan, its ADRs, and the roadmap's update for review.

The plan lands on the main branch in a pull request of its own, before the implementation, because later sessions start from the main branch and must be able to read it.
New ADRs land as Proposed.

## Working in a phase

Treat the phase plan as the instruction: implement toward its Done when items and verify with the checks they name.
The plan has already settled requirements, assumptions, decisions, and what counts as done, so none of those is by itself a reason to enter plan mode.
Enter plan mode only when how to build it is still open:

- several ways to build it remain within the decisions,
- the change reorganizes existing structure or spans many files,
- or the code is unfamiliar.

A plan-mode plan covers only how to build.
If it would change a requirement, an assumption, a decision, a dependency, or a Done when item, or it contains a decision the person should review with care, move that into the phase plan or an ADR, commit it, and get it reviewed before implementing on top of it.
The approval screen is the right place to review how; it is not a record, so it is the wrong place to settle what.

### When to go back to the plan

Go back when the plan itself stops holding, which its own sections tell you:

- an assumption turns out false, including when a risk's warning sign appears;
- a requirement, or the check a Done when item names, cannot hold as written;
- a decision proves unworkable, or the work needs a dependency the plan does not list.

Difficulty that stays within the plan's latitude is not a reason; solve it there.
When one of these happens, stop implementing, tell the person which assumption, requirement, or decision broke and what you observed, and follow [Changing direction](#changing-direction).
If you notice it in plan mode, say so rather than presenting a plan that routes around it.

## Changing direction

Revise the phase plan and commit the revision, so the diff shows what changed.
If the direction itself changes, research again and write a new ADR that supersedes the old one; the old one's status line is the only part of it that changes.

## Closing a phase

1. Under each Done when item, add the evidence that it holds: the command and its output, a screenshot, a link to a run.
2. Update the architecture overview for what the phase changed. For the first phase, this is usually where it is first written.
3. Settle each of the phase's ADRs: Accepted, or superseded.
4. Mark the phase done in the roadmap and the plan closed; the plan is not edited after this.

These changes go in the pull request that completes the last Done when item, or in one of their own if the implementation has already merged.

## Research

Research happens at two points: while writing a phase plan, for questions that could change direction, and during a work session, for how to implement, in plan mode if that helps.
Keep it in a working copy where you can commit, since some session recorders capture a conversation only when a commit follows it.

Place each result by how long it stays useful:

- **The process**: stays with the branch, not the main branch.
- **A finding a decision rests on**: goes into that ADR's context, with its source and the date you checked it.
- **A finding only this phase uses**: goes into the phase plan, as a sourced assumption.

Research notes are working files and never reach the main branch.
Commit them to the branch only if they are fit to publish wherever the branch is pushed, because a pull request keeps every commit reachable after its branch is deleted, and on a public host those commits are public.
Otherwise keep them untracked.
Remove any committed notes before the review, once their conclusions are in the ADR or the plan, so the review and the merged diff see only those.

If the repository has an `.entire/` directory, read [references/entire.md](references/entire.md) before researching.

## Work outside the roadmap

Track it where the repository's instructions say; without such an instruction, use the repository's issues.
Whether work that should stay private goes to a private tracker is the repository's choice.

- **A fix whose change one sentence can describe**: no document. Record it in the tracker if it needs tracking, and close it with the pull request.
- **A bug that needs investigation**: the report and the investigation go in the tracker, the fix in a pull request.
  - A decision that comes out of it gets an ADR, phase or not.
  - A change to structure or invariants updates the architecture overview.
- **Work that needs steering**, meaning its requirements must be decided and it spans sessions: insert it into the roadmap as a phase and write a phase plan.

## Specifications

Do not write one by default.
When a boundary gains dependents outside it, such as a file format, an API, or a command-line interface others build on, propose a specification of that boundary alone, agree its location with the person, and link it from the architecture overview.

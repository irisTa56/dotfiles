# Phase plan

A phase plan is for the person steering the phase: it fixes what the phase must achieve, what it takes for granted, what was decided, and how anyone will know it is done.
It leaves how to build to the session that builds it, so it has no implementation steps and no per-task test list.
A model that can read the code does not need them, and a person reading the plan is slowed down by them.
The test for any line is who needs it: it stays when the person needs it to see the edge of what is delivered or to know the phase is done, and it goes to the notes for building ([SKILL.md](../SKILL.md#research)) when the building session would settle or find it while building.
A true statement is held to that test as well, so a review finding about how a tool behaves adds a line only where it moves that edge.

Use bullets and subheadings, not tables: a table cell cannot wrap a sentence that grows.
Give requirements and assumptions short IDs so Done when can point at them.
The IDs are local to the plan.

## Skeleton

```markdown
# Phase NN: <name>

- Status: In progress
- Roadmap: [phase NN](../ROADMAP.md#phases)

## Goal

<goal>

## Requirements & Constraints

- **R001**: <requirement>
- **R002**: <constraint>

### Out of scope

- <left out>

## Assumptions & Risks

- **A001**: <assumption> Source: [<name>](https://…), checked YYYY-MM-DD.
  - Risk: <what breaks if it is false>, noticed by <observation>.
- **A002**: <assumption> Unverified.
  - Risk: <what breaks if it is false>, noticed by <observation>.

## Decisions

- [NNNN. <title>](../decisions/NNNN-<slug>.md)
- <X rather than Y, because Z.>

## Dependencies

- **<name>**: <purpose, and how it is installed or pinned>

## Done when

- **<what is observed>** — verifies R001, A002.
  - Check: <how it is observed>

## Open questions

- <question>
```

## What each section holds

- **Goal**: one or two sentences on what the phase establishes.
- **Requirements & Constraints**: the boundary conditions of what the phase delivers, which the person reads to see the edges of the deliverable before any code exists.
  - A requirement is an outcome the phase must deliver, stated so it can be observed.
  - A constraint limits how, such as "runs offline".
  - A limit a dependency puts on the outcome belongs to the requirement it narrows: say in one line what is not delivered because of it, with the cases known as examples, rather than listing how the dependency behaves.
  - **Out of scope**: what this phase deliberately leaves out, and where it goes instead if anywhere.
- **Assumptions & Risks**: each fact the plan relies on, with where it came from or the word Unverified.
  - A fact you verified is listed like one you did not, since the plan breaks the same way once it stops holding.
  - Under each, the risk: what breaks if it is false, and the observation that would reveal it.
    - The observation is the building session's signal to stop and come back to the plan, which is why the line names a sign rather than a countermeasure.
  - An unverified assumption the phase cannot proceed without is a question to settle now, not later.
- **Decisions**: a link to each ADR, and a line for each decision kept in the plan, in the form "X rather than Y, because Z".
  - A choice of how to build, such as the options a command runs with, is not kept here, even where the research already made it.
- **Dependencies**: only what the phase adds, since each one adds complexity and risk the person should see.
  - Where the exact package is chosen while building, name the purpose, so that a dependency for any other purpose reads as a change to the plan.
- **Done when**: what someone observes, not what was built, with the R and A items it verifies.
  - Write "the page shows the area coloured by last-edit date" rather than "the renderer is implemented", since the phase is done when its outcomes are observable, not when its tasks are complete.
  - The Check says how someone outside the code can observe it, such as a command and its expected output, or a page and what it shows.
    - It stops at what is observed and by what kind of check, and leaves which cases a fixture holds to the building session.
  - Every R and every unverified A appears in at least one item, or the plan says why not.
- **Open questions**: only questions whose answer will not change this plan.
  - One that could change a requirement, a decision, or a Done when item cannot wait, so resolve it before committing the plan.

## Closing

When the phase closes, add the evidence under each Done when item, set the status to `Closed YYYY-MM-DD`, and do not edit the plan again:

```markdown
- **<what is observed>** — verifies R001, A002.
  - Check: <how it is observed>
  - Evidence: <the test that verifies it and its result, or a command and its output, a screenshot, a link>
```

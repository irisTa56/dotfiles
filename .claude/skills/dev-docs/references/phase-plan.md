# Phase plan

A phase plan is for the person steering the phase: it fixes what the phase must achieve, what it takes for granted, what was decided, and how anyone will know it is done.
It leaves how to build to the session that builds it, so it has no implementation steps and no per-task test list.
A model that can read the code does not need them, and a person reading the plan is slowed down by them.

Use bullets and subheadings, not tables: a table cell cannot wrap a sentence that grows.
Give requirements and assumptions short IDs so Done when can point at them.
The IDs are local to the plan.

```markdown
# Phase NN: <name>

- Status: In progress
- Roadmap: [phase NN](../ROADMAP.md#phases)

## Goal

<One or two sentences on what the phase establishes.>

## Requirements & Constraints

- **R001**: <an outcome the phase must deliver, stated so it can be observed.>
- **R002**: <a constraint on how, such as "runs offline".>

### Out of scope

- <What this phase deliberately leaves out, and where it goes instead if anywhere.>

## Assumptions & Risks

- **A001**: <a fact the plan relies on.> Source: [<name>](https://…), checked YYYY-MM-DD.
  - Risk: <what breaks if it is false>, noticed by <the observation that would reveal it>.
- **A002**: <a fact> Unverified.
  - Risk: …

## Decisions

- [NNNN. <title>](../decisions/NNNN-<slug>.md)
- <X rather than Y, because Z.>

## Dependencies

- **<name>**: <what it is for, and how it is installed or pinned.>

## Done when

- **<what is observed>** — verifies R001, A002.
  - Check: <how someone outside the code can observe it, such as a command and its expected output, or a page and what it shows.>

## Open questions

- <Only questions whose answer will not change this plan.>
```

- A Done when item states what someone observes, not what was built: "the page shows the area coloured by last-edit date" rather than "the renderer is implemented".
  - Completing the tasks is not the goal, since the phase is done when its outcomes are observable.
- Every R and every unverified A appears in at least one Done when item, or the plan says why not.
- An assumption either cites where it came from or says Unverified.
  - An unverified one that the phase cannot proceed without is a question to settle now, not later.
- An Open question that could change a requirement, a decision, or a Done when item cannot wait, so resolve it before committing the plan.
- Dependencies lists only what the phase adds, since each one adds complexity and risk the person should see.
  - Where the exact package is chosen while building, name the purpose, so that a dependency for any other purpose reads as a change to the plan.
- When the phase closes, add the evidence under each Done when item, set the status to `Closed YYYY-MM-DD`, and do not edit the plan again:

  ```markdown
  - **<what is observed>** — verifies R001, A002.
    - Check: …
    - Evidence: `mise run build && open dist/index.html` showed … ([screenshot](…)).
  ```

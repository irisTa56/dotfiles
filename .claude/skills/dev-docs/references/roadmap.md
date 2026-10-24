# Roadmap

The roadmap says why the project exists, what it covers, and in what order the phases go.
It stays at the granularity of ideas, so it is short and changes only when direction does.
What a phase must achieve belongs to its plan, and why a choice was made belongs to an ADR, so neither is repeated here.

## Skeleton

```markdown
# Roadmap

## Purpose

<purpose>

## Scope

- <scope item>
- <scope item> [NNNN]

## Non-goals

- **<non-goal>**: <why>

## Phases

- **Phase 01: <name>**: <what it establishes> — [plan](plan/phase-01-<slug>.md), done
- **Phase 02: <name>**: <what it establishes> — [plan](plan/phase-02-<slug>.md), in progress
- **<name>**: <what it establishes>

[NNNN]: decisions/NNNN-<slug>.md
```

## What each section holds

- **Purpose**: one paragraph on who has what problem, and what this project does about it.
- **Scope**: what the project covers, as ideas rather than features to verify.
  - Write a requirement as the outcome someone needs, not as a mechanism: "a mapper can see which features have gone longest without an edit", not "colour features by timestamp".
  - A constraint that shapes every phase, such as data that must stay out of a public repository, goes here next to what it constrains.
- **Non-goals**: each thing left out, with why in one sentence.
- **Phases**: each phase in its planned order, with one sentence on what it establishes, a link to its plan once it has one, and its state.
  - Use a bullet list, not a numbered one, and show a phase's number only once it has started, since it gets its number then and a list numeral would give an unstarted phase one it may never have.
  - Refer to a phase that has not started by its name.
  - Say which part of the order is fixed and which is decided later, when that is so.

A choice the roadmap itself makes, such as a scope item, a non-goal, or the phase order, gets an ADR by the test in [adr.md](adr.md), written with the roadmap change that makes it.
The statement of that choice carries the link `[NNNN]`, and [adr.md](adr.md#where-it-is-linked-from) says how to write it and what happens to it when the statement goes or the decision stops holding.
A non-goal whose whole reason fits its one sentence needs no ADR.

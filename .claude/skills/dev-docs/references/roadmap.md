# Roadmap

The roadmap says why the project exists, what it covers, and in what order the phases go.
It stays at the granularity of ideas, so it is short and changes only when direction does.
What a phase must achieve belongs to its plan, and why a choice was made belongs to an ADR, so neither is repeated here.
It holds only the research findings its own content rests on; a finding behind a phase's decision or assumption goes where the skill's Research section says.

## Skeleton

```markdown
# Roadmap

## Purpose

<purpose>

## Background

- <finding> ([<source>](https://…), checked YYYY-MM-DD)

## Scope

- <scope item>

## Non-goals

- **<non-goal>**: <why>

## Phases

- **Phase 01: <name>**: <what it establishes> — [plan](plan/phase-01-<slug>.md), done
- **Phase 02: <name>**: <what it establishes> — [plan](plan/phase-02-<slug>.md), in progress
- **<name>**: <what it establishes>
```

## What each section holds

- **Purpose**: one paragraph on who has what problem, and what this project does about it.
- **Background**: each finding that the purpose, the scope, a non-goal, or the phase order rests on, with its source and the date it was checked.
  - When revising the roadmap, check again each finding the revision relies on, and update its date.
- **Scope**: what the project covers, as ideas rather than features to verify.
  - Write a requirement as the outcome someone needs, not as a mechanism: "a mapper can see which features have gone longest without an edit", not "colour features by timestamp".
  - A constraint that shapes every phase, such as data that must stay out of a public repository, goes here next to what it constrains.
- **Non-goals**: each thing left out, with why in one sentence.
- **Phases**: each phase in its planned order, with one sentence on what it establishes, a link to its plan once it has one, and its state.
  - Use a bullet list, not a numbered one, and show a phase's number only once it has started, since it gets its number then and a list numeral would give an unstarted phase one it may never have.
  - Refer to a phase that has not started by its name.
  - Say which part of the order is fixed and which is decided later, when that is so.

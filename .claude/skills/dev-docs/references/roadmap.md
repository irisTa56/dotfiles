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

## Non-goals

- **<non-goal>**: <why>

## Phases

1. **<phase name>**: <what it establishes> — [plan](plan/phase-01-<slug>.md), done
2. **<phase name>**: <what it establishes> — in progress
3. **<phase name>**: <what it establishes>
```

## What each section holds

- **Purpose**: one paragraph on who has what problem, and what this project does about it.
  - A finding that motivates the purpose may stay here in a sentence with its source.
  - A finding behind a decision goes beside that decision instead, as the skill's Research section says.
- **Scope**: what the project covers, as ideas rather than features to verify.
  - Write a requirement as the outcome someone needs, not as a mechanism: "a mapper can see which features have gone longest without an edit", not "colour features by timestamp".
  - A constraint that shapes every phase, such as data that must stay out of a public repository, goes here next to what it constrains.
- **Non-goals**: each thing left out, with why in one sentence.
- **Phases**: each phase in order, with one sentence on what it establishes, a link to its plan once it has one, and its state.
  - Say which part of the order is fixed and which is decided later, when that is so.

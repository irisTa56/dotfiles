# Roadmap

The roadmap says why the project exists, what it covers, and in what order the phases go.
It stays at the granularity of ideas, so it is short and changes only when direction does.
What a phase must achieve belongs to its plan, and why a choice was made belongs to an ADR, so neither is repeated here.

```markdown
# Roadmap

## Purpose

<One paragraph: who has what problem, and what this project does about it.>

## Scope

- <What the project covers, as ideas rather than features to verify.>

## Non-goals

- **<Thing left out>**: <why, in one sentence.>

## Phases

1. **<Phase name>**: <what it establishes, in one sentence.> — [plan](plan/phase-01-<slug>.md), done
2. **<Phase name>**: <one sentence.> — in progress
3. **<Phase name>**: <one sentence.>
```

- Write a requirement as the outcome someone needs, not as a mechanism: "a mapper can see which features have gone longest without an edit", not "colour features by timestamp".
- A constraint that shapes every phase, such as data that must stay out of a public repository, goes in Scope next to what it constrains.
- Say which part of the phase order is fixed and which is decided later, when that is so.
- A finding that motivates the purpose may stay here in a sentence with its source.
  - A finding behind a decision goes beside that decision instead, as the skill's Research section says.

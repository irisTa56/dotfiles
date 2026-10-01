# Architecture overview

The overview gives a newcomer the map of the code as it is now, following [matklad's ARCHITECTURE.md](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html).
Every recurring contributor reads it, so keep it short and keep to what is unlikely to change often.
It is written when the first phase closes and revised when a phase, or a change outside one, alters structure or invariants.

## Skeleton

```markdown
# Architecture

## Bird's-eye view

<overview>

## Code map

### `<module or directory>`

<what lives here>

## Invariants

- <invariant>
- <invariant> [NNNN]

## Boundaries and outside-visible behavior

- <boundary>
- <outside-visible behavior>

## Cross-cutting concerns

- <concern>

[NNNN]: decisions/NNNN-<slug>.md
```

## What each section holds

- **Bird's-eye view**: the problem, and how the system solves it, in a paragraph.
- **Code map**: for each module or directory, what lives there and how it relates to its neighbours.
  - Name the important files and types rather than linking them, since links go stale.
  - Where the repository runs a link checker over local links, link them instead and let the checker catch staleness.
- **Invariants**: what always holds, including what is deliberately absent, such as "the renderer never reads the network".
- **Boundaries and outside-visible behavior**:
  - Each boundary between layers or systems, and what crosses it.
  - Each behavior someone outside the code can observe or depend on, in a line, linking a specification where one exists.
- **Cross-cutting concerns**: error handling, configuration, testing, and the like, where they follow one rule across the code.

A statement in any section that rests on an accepted ADR carries the link `[NNNN]`, as [adr.md](adr.md#where-it-is-linked-from) describes, and when the statement goes, the link and its definition go with it and the ADR's status changes as described there.

Describe the present, not the history or the plan, which the ADRs and the plans already hold.

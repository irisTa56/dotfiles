# Architecture overview

The overview gives a newcomer the map of the code as it is now, following [matklad's ARCHITECTURE.md](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html).
Every recurring contributor reads it, so keep it short and keep to what is unlikely to change often.
It is written when the first phase closes and revised when a phase, or a change outside one, alters structure or invariants.

```markdown
# Architecture

## Bird's-eye view

<The problem, and how the system solves it, in a paragraph.>

## Code map

### `<module or directory>`

<What lives here and how it relates to its neighbours. Name the important files and types.>

## Invariants

- <Something that always holds, including what is deliberately absent, such as "the renderer never reads the network".>

## Boundaries and outside-visible behavior

- <A boundary between layers or systems, and what crosses it.>
- <Behavior someone outside the code can observe or depend on, in a line each. Link a specification where one exists.>

## Cross-cutting concerns

- <Error handling, configuration, testing, and the like, where they follow one rule across the code.>

## Decisions

- [NNNN. <title>](decisions/NNNN-<slug>.md)
```

- Describe the present, not the history or the plan; the history is in the ADRs and the plans.
- Name files and types rather than linking them, since links go stale — unless the repository runs a link checker over local links, in which case link them and let the checker catch staleness.
- List only accepted ADRs that still shape the code, and drop a superseded one in favour of its successor.

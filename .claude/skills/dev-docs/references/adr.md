# Decision record (ADR)

An ADR keeps one decision together with what it rested on and what it turned down, so a later reader can tell whether the reasons still hold.
Write one when someone changing that part later would need the reason: the decision is costly to reverse, or the code does not show why it was made.

## Skeleton

```markdown
# NNNN. <decision>

- Status: Proposed
- Date: YYYY-MM-DD

## Context

<context>

## Decision

<decision in full>

## Rejected alternatives

- **<alternative>**: <why it lost>

## Consequences

- **Dependencies added**: <dependencies>
- **Risks**: <risks>
```

## What each part holds

- **Title**: the decision, stated as what was chosen.
- **Context**: the problem and the forces on it.
  - Each finding carries its source and the date it was checked, as in `Geofabrik extracts keep each object's timestamp ([technical notes](https://…), checked 2026-09-28)`.
  - A source can also be a path in the repository or a command you ran.
- **Decision**: what was chosen, in a few sentences.
- **Rejected alternatives**: each alternative and why it lost, pointing at the context where it can.
  - Keep an alternative only if it was a real option, since listing strawmen hides the ones that mattered.
  - These are what turn a later review into a choice between options rather than a yes or no, so do not drop them to save space.
- **Consequences**: each dependency the decision adds, or "none", and what could go wrong because of it and how that would show.

## Status and superseding

- The status is one of `Proposed`, `Accepted`, `Rejected`, and `Superseded by [NNNN](NNNN-<slug>.md)`.
  - An ADR written at the start of a phase is Proposed and becomes Accepted when the phase closes.
  - One written with the change that implements it, as in a bug fix, lands Accepted.
  - A decision dropped before its pull request merges is simply not merged.
  - A Proposed decision dropped after it merged, with nothing replacing it, becomes Rejected, with a line in its Context saying why.
- To change a decision, write a new ADR whose Context says what changed and begins with `Supersedes [NNNN](NNNN-<slug>.md)`.
  - Then change the old one's status line and nothing else.

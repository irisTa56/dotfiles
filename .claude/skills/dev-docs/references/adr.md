# Decision record (ADR)

An ADR keeps one decision together with what it rested on and what it turned down, so a later reader can tell whether the reasons still hold.
Write one when someone changing that part later would need the reason: the decision is costly to reverse, or the code does not show why it was made.
Any other decision is a line in its phase plan instead, however much research it took.

```markdown
# NNNN. <Decision, stated as what was chosen>

- Status: Proposed
- Date: YYYY-MM-DD

## Context

<The problem and the forces on it. Each finding carries its source and the date it was checked, as in "Geofabrik extracts keep each object's timestamp ([technical notes](https://…), checked 2026-09-28)". A source can also be a path in the repository or a command you ran.>

## Decision

<What was chosen, in a few sentences.>

## Rejected alternatives

- **<Alternative>**: <why it lost, pointing at the context where it can.>

## Consequences

- **Dependencies added**: <each new dependency, or "none".>
- **Risks**: <what could go wrong because of this choice, and how it would show.>
```

- The status is one of `Proposed`, `Accepted`, and `Superseded by [NNNN](NNNN-<slug>.md)`.
  - An ADR written at the start of a phase is Proposed and becomes Accepted when the phase closes.
  - One written with the change that implements it, as in a bug fix, lands Accepted.
  - A decision dropped before its pull request merges is simply not merged.
- To change a decision, write a new ADR whose Context says what changed and begins with `Supersedes [NNNN](NNNN-<slug>.md)`, then change the old one's status line and nothing else.
- Keep an alternative only if it was a real option, since listing strawmen hides the ones that mattered.
- The rejected alternatives are what turn a later review into a choice between options rather than a yes or no, so do not drop them to save space.

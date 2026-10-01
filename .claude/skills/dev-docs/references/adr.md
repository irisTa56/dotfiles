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

- The status is one of `Proposed`, `Accepted`, `Rejected`, `Deprecated`, and `Superseded by [NNNN](NNNN-<slug>.md)`.
  - An ADR written within a phase, at its start or midway, is Proposed and is settled when the phase closes.
  - One written outside any phase lands Accepted with the change it belongs to, such as the bug fix that implements it or the roadmap change that states it.
  - A decision dropped before its pull request merges is simply not merged.
  - A Proposed decision dropped after it merged, with nothing replacing it, becomes Rejected, with a line in its Context saying why.
  - An Accepted decision that stops holding, with nothing replacing it, becomes Deprecated, with a line in its Context saying why.
- To change a decision, write a new ADR whose Context says what changed and begins with `Supersedes [NNNN](NNNN-<slug>.md)`.
  - Then change the old one's status line and nothing else.

## Where it is linked from

- A phase plan's Decisions links each ADR the phase wrote, and keeps that link once the plan is closed.
- The roadmap and the architecture overview link an ADR from each statement it supports, so a reader sees which decision a statement rests on.
  - Write the link as a shortcut reference after the statement, `[NNNN]`, and define it at the foot of the file as `[NNNN]: decisions/NNNN-<slug>.md`.
  - Every Accepted ADR is linked this way, so where neither document has a statement it supports, add the statement.
  - When an ADR supersedes it, the link moves to the successor.
  - When the statement goes and nothing supersedes the decision, the link and its definition go with it, and the ADR becomes Deprecated.

---
description: Readability rules for markdown prose.
paths:
  - "**/*.md"
---

# Readable Writing

When writing or editing prose in markdown documents, follow these rules.

## Say it once, in the right place

- Detailed explanation at each relevant point is fine as long as it fits the local context.
- What to avoid is near-identical wording appearing in multiple places — if two passages read almost the same, one of them should be trimmed.
- Where detailed elaboration is not the focus of the current passage, keep the mention brief so it does not interrupt the reader's flow.

## Keep lines short and scannable

- Break at Japanese sentence boundaries (句点 `。` or bullet boundaries) so that each line carries one idea.
- When a single bullet grows beyond roughly two clauses, consider nesting rather than simply inserting line breaks.
  - Simply splitting a long bullet with line breaks is not enough.
  - Consider whether the content has a parent-child relationship that warrants nesting.
  - A plain line break is fine when nesting adds no clarity, and no break at all is fine when the line is already short.
- Prefer a vertical list over a long inline enumeration (e.g., `A、B、C、D` → four sub-bullets).

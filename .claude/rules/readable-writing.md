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

- Do not insert a line break inside a sentence. Line breaks are allowed only at sentence boundaries (句点 `。`, period `.`, or bullet boundaries).
  - An in-sentence break renders inconsistently — some renderers soft-wrap it away, others show a hard break — so it is the worst way to shorten a line. Never reach for it as a fix.
- A long or multi-clause bullet is a structural problem. Fix the structure; do not paper over it with a line break. If a bullet carries two or more clauses, take one of the following:
  - Rewrite them into a single sentence (paraphrasing or joining with `;`).
  - Nest them as child bullets (parent-child).
  - Separate them as sibling bullets.
- Restructuring may reach beyond the offending bullet: promote, demote, or regroup neighboring bullets so the hierarchy reflects the actual structure. Reorganizing the surrounding list is expected — do not settle for a local edit that leaves the surrounding hierarchy wrong.
- Do not write a long inline enumeration. Expand `A、B、C、D` into a vertical list of sub-bullets.

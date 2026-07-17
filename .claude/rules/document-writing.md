---
description: How to write clear prose — substance, sourcing, and readability
paths:
  - "**/*.md"
---

# Writing Prose

These rules apply to prose written to a file, and to substantial prose in conversation; the precise trigger lives in `~/.claude/INSTRUCTIONS.md`. Behavioral principles (honesty, grounding claims) live there; this file covers prose-authoring specifics.

## Substance and logic

- The purpose of writing is to convey a claim. A claim must have a clear direction — a stance or implication, not a neutral pile of facts.
- For multi-section documents, maintain a coherent narrative: the reader should be able to follow why each section exists.
- Develop the argument step by step. The reader must never feel a logical leap — they should never suspect the author does not actually understand what they wrote.

## Sources

- When a statement relies on information that is not common knowledge, link the relevant phrase to a primary or authoritative source.
  - Common knowledge (no link): widely known facts, basic language/framework features documented in every tutorial.
  - Non-obvious (link needed): specific API behavior, numeric claims (benchmarks, statistics, thresholds), design rationale behind a tool, changes introduced in a particular version, conclusions from a paper or blog post.
- Use inline markdown links — link the relevant phrase rather than appending a bare URL.
  - NG: `The library uses epoll internally.`
  - OK: `The library [uses epoll internally](https://docs.example.com/internals#event-loop).`
- If you fetched or consulted a URL while composing the text, cite it — omitting it is the most common violation of this rule.
- When multiple statements draw from the same source, a single link on first mention is sufficient.

## Say it once, in the right place

- Detailed explanation at each relevant point is fine as long as it fits the local context.
- Avoid near-identical wording appearing in multiple places — if two passages read almost the same, trim one of them.
- Where detailed elaboration is not the focus of the current passage, keep the mention brief so it does not interrupt the reader's flow.

## Keep lines short and scannable

- Do not insert a line break inside a sentence. Line breaks are allowed only at sentence boundaries (period `.`, 句点 `。`, or bullet boundaries).
  - An in-sentence break renders inconsistently — some renderers soft-wrap it away, others show a hard break — so it is the worst way to shorten a line. Never reach for it as a fix.
- Do not mimic the wrap width of surrounding hard-wrapped prose; apply the one-sentence-per-line rule regardless of how the neighboring lines happen to wrap. Pattern-matching the local line width is a classic misjudgment.
- A long or multi-clause bullet is a structural problem. Fix the structure; do not paper over it with a line break. If a bullet carries two or more clauses, take one of the following:
  - Rewrite them into a single sentence (paraphrasing or joining with `;`).
  - Nest them as child bullets (parent-child).
  - Separate them as sibling bullets.
- Restructuring may reach beyond the offending bullet: promote, demote, or regroup neighboring bullets so the hierarchy reflects the actual structure. Reorganizing the surrounding list is expected — do not settle for a local edit that leaves the surrounding hierarchy wrong.
- Do not write a long inline enumeration. Expand `A, B, C, D` into a vertical list of sub-bullets.

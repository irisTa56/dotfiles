---
description: Honesty and logical rigor for technical writing
paths:
  - "**/*.md"
---

# Honest Technical Writing

When writing or editing prose in markdown documents, follow these rules.

## Core principle

- The purpose of writing is to convey a claim. A claim must have a clear direction (a stance or implication, not a neutral pile of facts), and it must be supported by both logical development and factual evidence.
- The Substance and Logic rules below exist to enforce these two kinds of support.

## Substance

- For multi-section documents, maintain a coherent narrative — the reader should be able to follow why each section exists.
- Avoid ornate language; explain things honestly.
  - Prioritize objective facts. When you are not certain, say so plainly instead of hedging or asserting.
  - Do not use exaggeration, sweeping assertions, promotional language, praise, or theatrical phrasing.
  - To remove AI-sounding patterns from prose, use the `humanizer` skill as a reference.

## Sources

- When a statement relies on information that is not common knowledge, link to a primary or authoritative source so the reader can verify it.
  - Common knowledge (no link needed): widely known facts, basic language/framework features documented in every tutorial.
  - Non-obvious (link needed): specific API behavior, numeric claims (benchmarks, statistics, thresholds), design rationale behind a tool, changes introduced in a particular version, conclusions from a paper or blog post.
- Use inline markdown links — link the relevant phrase rather than appending a bare URL.
  - NG: `The library uses epoll internally.`
  - OK: `The library [uses epoll internally](https://docs.example.com/internals#event-loop).`
- If you fetched or consulted a URL while composing the text, that URL is almost certainly worth citing — omitting it is the most common violation of this rule.
- When multiple statements draw from the same source, a single link on first mention is sufficient.

## Logic

- Develop the argument carefully and step by step.
- The reader must never feel a logical leap — that is, they should never suspect the author does not actually understand what they wrote.

---
description: Review Elixir code against the anti-patterns in Elixir's guides
paths:
  - "**/*.ex"
  - "**/*.exs"
---

# Elixir

Follow these rules when reviewing Elixir.

## Anti-patterns

- Check the code against every anti-pattern on the four pages of Elixir's guides that follow [What are anti-patterns?](https://elixir.hexdocs.pm/main/what-anti-patterns.html):
  - [code-related](https://elixir.hexdocs.pm/main/code-anti-patterns.html)
  - [design-related](https://elixir.hexdocs.pm/main/design-anti-patterns.html)
  - [process-related](https://elixir.hexdocs.pm/main/process-anti-patterns.html)
  - [meta-programming](https://elixir.hexdocs.pm/main/macro-anti-patterns.html)
- Read the pages rather than working from memory, since each entry's own text sets where it applies and what it exempts; where they cannot be fetched, say so in the review.
- Raise a match even where it does no visible harm.
  - Where some required reads are assertive and others are not, a non-assertive read stops telling the reader that the value is optional, and the one that hides a real bug passes unnoticed among them.
- Accept a match only where a comment beside it states why: an exception the page names, or a constraint the rewrite would break.

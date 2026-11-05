---
description: Hold Elixir code to the anti-patterns in Elixir's guides, when writing it and when reviewing it
paths:
  - "**/*.ex"
  - "**/*.exs"
---

# Elixir

Follow these rules when writing or reviewing Elixir.

## Anti-patterns

- Check the code against every anti-pattern in Elixir's guides: the code, design, process and meta-programming pages, which [What are anti-patterns?](https://elixir.hexdocs.pm/main/what-anti-patterns.html) opens and each page's next-page link continues.
  - Read the pages rather than working from memory, since each entry's own text sets where it applies and what it exempts.
- Rewrite a match even where it does no visible harm.
  - Where some required reads are assertive and others are not, a non-assertive read stops telling the reader that the value is optional, and the one that hides a real bug passes unnoticed among them.
- Leave a match in place only with a comment beside it that states why: an exception the page names, or a constraint the rewrite would break.

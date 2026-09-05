---
description: How to write clear prose — substance, sourcing, and readability
paths:
  - "**/*.md"
---

# Writing Prose

These rules apply to prose you write, and to substantial prose in conversation; the precise trigger lives in `~/.claude/INSTRUCTIONS.md`.
Behavioral principles (honesty, grounding claims) live there; this file covers prose-authoring specifics.

Apply them only to what you wrote. Do not sweep unrelated existing passages into the rewrite.

## Substance and logic

- The purpose of writing is to convey a claim. A claim must have a clear direction — a stance or implication, not a neutral pile of facts.
- For multi-section documents, maintain a coherent narrative: the reader should be able to follow why each section exists.
- Develop the argument step by step. The reader must never feel a logical leap — they should never suspect the author does not actually understand what they wrote.
- Do not simplify through omission, and do not reach for a metaphor the reader cannot resolve; say it with a plain verb.

## Sources

- When a statement relies on information that is not common knowledge, link the relevant phrase to a primary or authoritative source.
  - Common knowledge (no link): widely known facts, basic language/framework features documented in every tutorial.
  - Non-obvious (link needed): specific API behavior, numeric claims (benchmarks, statistics, thresholds), design rationale behind a tool, changes introduced in a particular version, conclusions from a paper or blog post.
- Use inline markdown links — link the relevant phrase rather than appending a bare URL.
  - NG: `The library uses epoll internally.`
  - OK: `The library [uses epoll internally](https://docs.example.com/internals#event-loop).`
- If you fetched or consulted a URL while composing the text, cite it — omitting it is the most common violation of this rule.
- When multiple statements draw from the same source, a single link on first mention is sufficient.
- When citing a place in a file the same work is still editing, anchor on the quoted text rather than a line number; the edit that writes the citation is usually the edit that moves the line.

## Say it once, in the right place

- Detailed explanation at each relevant point is fine as long as it fits the local context.
- Avoid near-identical wording appearing in multiple places — if two passages read almost the same, trim one of them.
- Where detailed elaboration is not the focus of the current passage, keep the mention brief so it does not interrupt the reader's flow.
- Where another file owns some content and keeps it updated, do not restate that content; carry a pointer and whatever yours adds.
  - Attributing a claim of your own is `## Sources` above, not a restatement this bars; reproducing what the other file maintains as its content is.
  - The copy is what goes wrong, not the extra words: it stops matching, and a reader with it in front of them acts on it instead of going to the source.
    - Declaring that the source takes precedence does not prevent this.
  - Carry the content as well as the pointer wherever the pointer alone would leave the reader worse off than a copy.
    - The reader cannot open the source — you are writing a prompt for an agent barred from reading it.
    - Opening the source would hand them more than they should act on — you are briefing a subagent to execute one slice of a parent prompt.
    - The destination's own convention requires the gist in the text, as a PR body's does (`pr-description`).

## Keep lines short and scannable

- Do not insert a line break inside a sentence. Line breaks are allowed only at sentence boundaries (period `.`, 句点 `。`, or bullet boundaries).
  - An in-sentence break renders inconsistently — some renderers soft-wrap it away, others show a hard break — so it is the worst way to shorten a line. Never reach for it as a fix.
- Break at every sentence boundary in a prose paragraph, so a paragraph is one sentence per line.
  - The rule above says where a break may go; this says where one must.
  - What it buys is the diff: a one-sentence edit is a one-line change, so a reviewer sees which sentence moved rather than a re-flowed paragraph.
  - Sentences may share a line where together they fit within 100 half-width characters, however many they are. A full-width character counts as two, so Japanese prose reaches the limit at 50. On a line that short, "this line changed" tells the reviewer as much as "this sentence changed" would, so the diff argument does not bite.
  - A paragraph is what the renderer flows into one block, where a newline reads as a space; this rule reaches nothing else. A bullet, a table cell and a YAML frontmatter scalar are not paragraphs, and a break inside one changes what it is rather than how it wraps — a newline at column 0 inside a `description:` stops the frontmatter parsing whether or not it is quoted, so the skill silently stops loading.
    - Where a bullet carries two sentences, `## Give a list the shape its content has` decides whether it becomes two bullets; adding a line break inside it is what that section forbids.
- Do not mimic the wrap width of surrounding hard-wrapped prose; apply the sentence-boundary rules above regardless of how the neighboring lines happen to wrap. Pattern-matching the local line width is a classic misjudgment.

## Give a list the shape its content has

- A list's shape is itself a claim: siblings assert the items are peers, and a child asserts it belongs to its parent. Write the shape the content has, not the flattest one that parses.
  - Where an item qualifies, elaborates, or is a case of another, nest it under that one. Leaving it a sibling asserts they are peers, which is false.
  - Where one bullet holds two things that are not one thought, they are two bullets.
- A flat list of short one-sentence bullets can still be wrong, so clause count is not the check. Ask of every list what nesting or splitting would say about it, and take that shape wherever it is the true one.
- A bullet that has grown long is the one signal you can see without asking, so read it as the prompt to re-examine the list's shape rather than as one bullet to shorten.
  - Fix the structure, and NEVER paper over it with a line break.
  - Restructuring may reach beyond that bullet: promote, demote, or regroup neighboring bullets so the hierarchy reflects the actual structure. Reorganizing the surrounding list is expected — do not settle for a local edit that leaves the surrounding hierarchy wrong.
- Do not write a long inline enumeration. Expand `A, B, C, D` into a vertical list of sub-bullets.

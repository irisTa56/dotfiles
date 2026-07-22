---
name: ask-kboat
description: >
  Answer a question using the K-Boat knowledge base (the `k-boat-knowledge` Basic Memory
  project) as the primary source — search the concept graph, read the notes that bear on
  the question, and ground the answer in what the reader has already distilled, saying
  plainly where that diverges from or is absent in general knowledge. Read-only: it never
  writes to the knowledge base. Invoke when the user explicitly asks for their own notes or
  knowledge base to be consulted — "KB を参照して答えて", "what do my notes say about X",
  "check my knowledge base first", "自分が読んだ内容だとどうなってる" — and not on ordinary
  factual questions, where searching the graph costs time for nothing.
---

# Ask the K-Boat knowledge base

K-Boat reads content through NotebookLM and distills it into concept notes.
Those notes are the reader's own accumulated understanding: what they actually read, what they questioned, and what they concluded.
This skill answers a question from that base rather than from general knowledge alone.

The value is not retrieval for its own sake. It is that the reader's own base is often **more specific, more current, and better reasoned** on their topics than a generic answer, because it was built from sources they chose and dialogue they drove. Where the two disagree, that disagreement is itself the interesting finding.

## Scope

Read-only. Search and read; never write, edit, or move a note.
Distillation into the base belongs to `kboat-distill`, and graph maintenance to `kboat-curate` — both deliberately separate, so consulting the base can never mutate it as a side effect.
If the answer turns up something worth recording, say so and let the user decide; do not record it.

## Procedure

Basic Memory's tools are built for a different job than this one — resuming recent work, which is what `memory-continue` does. Several of their defaults, and their behaviour when a call finds nothing, are wrong for asking timeless questions of a matured base. Each is called out below at the point where it bites.

Pass `project="k-boat-knowledge"` on **every** call: a default project set elsewhere must never answer in its place.

### 1. Search

Build the query from the concepts in the question, **in English** — the concept notes are written in English regardless of the language the user asks in. A Japanese question about ページング付き注意機構 has to be searched as `PagedAttention KV cache paging`.

```text
search_notes(query="<English concept terms>", project="k-boat-knowledge")
```

Semantic search is on by default, so **this call never comes back empty**. A nonsense string still returns a full page of notes. Hits are therefore not evidence of coverage on their own, and a page of uniformly weak matches is what silence actually looks like here.

Dressing those notes up as "what your base says" is **the failure this skill is built to prevent**: it leaves the base looking as though it covers ground it does not. It is also the easy mistake, because the results arrive looking like an answer.

Two things tell the cases apart:

- **Judge relevance by reading, not by rank.** Ask whether the note is about the question at all. This is what settles it, and it is what gets skipped when the ranking looks confident.
- **Read the shape of the distribution, never a threshold.** A genuine hit stands clear of the tail behind it; unrelated results arrive as a flat cluster with nothing rising above it. Do not turn that into a cutoff: the on-topic tail sits only a little above the noise, and both bands move with the embedding model, so a fixed number would discard real matches.

Before concluding the base is silent, work at it. Try the obvious alternates — the expanded form and the acronym, the vendor or library name, the adjacent concept. Concept notes are titled by the concept, so a title-shaped query usually beats a sentence-shaped one.

Then cross-check lexically with `search_type="text"`, **one term per call**. Unlike the semantic default it does return nothing for a term the base does not contain, which is what makes absence observable rather than inferred. But it ANDs its terms, so a multi-term query also comes back empty when the terms simply never co-occur in one note — hand it the phrase from the search above and it will report silence on concepts the base covers in depth. Only a single-term miss is evidence of absence.

### 2. Read

Read the notes that actually bear on the question, not everything that matched.

```text
read_note(identifier="<permalink>", project="k-boat-knowledge")
```

The `## Relations` section lists neighbours as wikilinks. Follow one when the question needs it — a question about a trade-off usually lives in the relation between two concepts rather than inside either note. `build_context` pulls a whole neighbourhood in one call:

```text
build_context(url="memory://<the permalink step 1 returned>",
              project="k-boat-knowledge", timeframe="1 year ago")
```

Paste the permalink from the search result verbatim; never compose the path from the concept's name. Notes sit under both `concepts/` and `meta/`, so a composed path is a guess — and `build_context` does not reject a URL that resolves to nothing. It falls back to something near it and reports success: a nonexistent slug comes back as an unrelated note with a full set of relations, which then reads as a confident answer about a concept the reader never asked after. Check that the primary in the response carries the permalink you asked for before you use it.

Set `timeframe` even though it has a default, because the default is the one that breaks this. It is `7d`, which filters neighbours by recency — right for resuming recent work, wrong here, where the neighbours worth having are precisely the older notes. Left alone the call returns `0 related` for a note whose `## Relations` lists several, and reports success while doing it. A year is the widest the tool accepts, so this narrows the filter rather than removing it: as the base ages past that, neighbours will start dropping out the same silent way. When the count looks short against the note's own `## Relations` list, fall back to `read_note` on the wikilinks.

### 3. Answer, and say what the base earned

Answer the question, using the notes as the primary source, in the language the user asked in.

Read observation tags as evidence strength, because they record where a claim came from:

- `#grounded` — the source supported it. Treat as the reader's verified material.
- `#dialogue` — surfaced in reading-time conversation, verified at distillation but not straight from the source. Still usable; worth attributing when it carries the answer, since the reader will recognise it as their own reasoning.

Only those two speak to evidence. An observation may carry further tags on the same line, and a note's frontmatter carries its own set; those categorise the subject and say nothing about how well-supported the claim is.

Cite the notes you used by title (permalink when the user may want to open one). The reader wants to know *which* of their notes answered, so they can go back to it.

Then state how much of the answer actually came from the base, following the section below. That is not a closing flourish: without it the citations above give the reader no way to tell their own knowledge from yours.

## Being honest about coverage

The reader cannot tell your general knowledge from their base unless you separate them. Keep the boundary visible.

First, a state that is not a coverage answer at all: if a Basic Memory call **errors**, the base was never consulted, and nothing about its contents follows. This is worth naming because the error does not identify itself — an unregistered project currently surfaces as `Cloud routing requested but no credentials found`, which mentions neither the project nor registration, and the nearest sensible-looking reading of it is "nothing came back." Report that the base is unreachable, name the likely cause (the `k-boat-knowledge` project is not registered here; `basic-memory project list` shows what is), and stop. Falling through to the silent branch would tell the reader their base lacks a topic it may in fact cover completely.

When the base did answer, place it in one of these:

- **The base answers.** Answer from it, cite the notes, and add general knowledge only as clearly-marked context.
- **The base is silent.** Say so in a sentence, then answer from general knowledge, marked as such. Left unmarked, this becomes the failure named in step 1 arriving from the other side: an answer that came from nowhere in the base still leaves the base looking broader than it is.
- **The base is partial.** Name which part it covers and which part it does not. This is the common case and the most useful one, because it shows the reader exactly where their next source should go.
- **The base and general knowledge disagree.** Surface the conflict rather than picking a winner silently. Either the base is stale, or the general knowledge is wrong or coarser, and only the reader can settle which. Give them the two positions and your read of why they differ.

Where the notes are richer or sharper than the generic answer — a concrete number, a measured trade-off, an implementation detail — lead with that.

## Tool behaviour this skill relies on

The claims above about how the tools behave — semantic search being the default, `search_type="text"` being a case-insensitive prefix match, conjunctive across terms, `build_context`'s `7d` window, its one-year ceiling, and its fallback on an unresolvable URL, the wording of the unregistered-project error, and the call signatures in the code blocks — were checked against Basic Memory 0.22.1 on 2026-07-22.
They are the parts of this skill that can rot without anything failing loudly, so re-test them after an upgrade rather than trusting this section.

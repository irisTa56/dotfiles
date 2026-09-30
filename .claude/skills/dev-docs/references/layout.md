# Default layout

Use this layout only in a repository that has no convention of its own for these documents.
`docs/` stays for users; everything for developers goes under `dev/`.

```text
dev/
  ROADMAP.md
  ARCHITECTURE.md
  plan/
    phase-01-<slug>.md
  decisions/
    0001-<slug>.md
  research/          # working notes; never on the main branch
```

- The top level of `dev/` holds the documents that keep being revised. The subdirectories hold records, numbered and frozen once closed or accepted.
- A slug names its phase or decision in a few lowercase, hyphenated words, as in `phase-01-local-map.md` or `0003-pmtiles-over-geojson.md`.
- Phases are numbered in the order they start, with two digits; decisions in the order they are written, with four. Numbers are never reused, including for a superseded ADR.
- Where the repository keeps research notes untracked, add `dev/research/` to `.git/info/exclude` so a broad `git add` cannot pick them up.
- Link the documents to each other with relative links, and link the root README to `dev/` so a reader can find them.

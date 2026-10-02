# Default layout

Use this layout only in a repository that has no convention of its own for these documents.
`docs/` stays for users, and everything for developers goes under `dev/`.

```text
dev/
  ROADMAP.md
  ARCHITECTURE.md
  plan/
    phase-01-<slug>.md
  decisions/
    0001-<slug>.md
```

- The top level of `dev/` holds the documents that keep being revised.
- The subdirectories hold records, numbered and frozen once closed or accepted.
- A slug names its phase or decision in a few lowercase, hyphenated words, as in `phase-01-local-map.md` or `0003-pmtiles-over-geojson.md`.
- A phase gets its number, two digits, when it starts, so a phase inserted ahead of planned ones takes the next number and the planned ones keep none until they start.
  - The roadmap's list order is the planned order, and a phase not yet started is known by its name.
- A decision gets its number, four digits as adr-tools and MADR use, when it is written.
- Numbers are never reused, including for a superseded ADR.
- Link the documents to each other with relative links, and link the root README to `dev/` so a reader can find them.

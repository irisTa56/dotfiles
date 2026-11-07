---
name: find-kboat-repo
description: "Find software for a need the user states in words — \"I want to do X\" — among the GitHub repositories they catalogued in K-Boat, and say plainly when the catalogue holds nothing that fits. Searches the catalogue only, never the web. Read-only; never writes to the vault."
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_SKILL_DIR}/scripts/export_catalog.sh)
---

# Find a repository in the K-Boat catalogue

K-Boat keeps one note per GitHub repository the user has come across, in the `Repos/` folder of their Obsidian vault.
This skill answers "is there something in my catalogue I could use for this?" from those notes.

The value is that every candidate is one the user already chose to keep.
So the catalogue is the whole search space: when nothing in it fits, say so and stop.
Do not widen to the web or to general knowledge of other projects, because an answer drawn from outside the catalogue reads as though the user had catalogued it.
Where the user then wants a wider search, they will ask for one.

## Scope

Read-only: never write, edit or move a note, and never run a `kboat-repos` command, all of which reach GitHub or the vault.
Catalogue maintenance belongs to K-Boat's own skills, run from the k-boat repository.

## Procedure

### 1. Restate the need

Write the need down as the judge will read it, in one or two sentences: what the user wants to do, and any constraint they named (language, runtime, licence, scale, "a library rather than an app").
Keep it in the user's terms and add nothing they did not say; a constraint you invent filters out what they might have taken.
If the need is too vague to judge a repository against — "something for data" — ask one question before searching.

### 2. Export the catalogue

Run `${CLAUDE_SKILL_DIR}/scripts/export_catalog.sh`.
It reads every repo note through K-Boat's `kboat-note list` into a new temporary directory and splits a compact copy (title, role, domain, topics, description, summary) into chunk files of about a hundred notes each.
It prints `{dir, notes, anomalies, chunks}`.

- A non-zero exit means the catalogue was not read, and nothing about its contents follows. Report the exit and any `anomalies` it printed, and stop rather than reporting that nothing fits.
  - The script finds k-boat at `~/Documents/_repos/k-boat`, or at `KBOAT_REPO` when set; a missing directory fails here.
- A non-empty `anomalies` on a zero exit names notes that could not be read. Carry on over the rest, and name each one beside the answer, because a repository the search did not see reads as one the user never catalogued.

The full export is too large to read into this conversation, which is why the next step hands it out in chunks.
Do not `cat` `catalog.json` or `compact.jsonl`.

### 3. Judge every chunk with a cheap subagent

Spawn one subagent per chunk, all in one message so they run in parallel, each with `model: "haiku"`.
Every subagent reads its whole chunk rather than a keyword-filtered list.
A keyword pass loses exactly the matches this skill exists for: the `summary` is Japanese while `description` and `topics` are English, and a need put in words often shares no term with the repository that answers it.

Give each subagent this prompt, filled in:

```text
Read the file <chunk path>. Each line is one GitHub repository from a personal
catalogue, as JSON with title, role, domain, topics, description and summary
(summary is in Japanese).

The need: <the restated need from step 1>

Judge each repository by whether the user could use it to meet this need: as a
library or tool they would adopt, an application they would run, or a recipe or
sample they would build from. A repository that is only on a related topic does
not count.

Return only the repositories that fit, one JSON object per line, nothing else:
{"title": "<the title exactly as the line gives it>", "fit": "direct" | "partial", "reason": "<one Japanese sentence on how it meets the need, or which part it meets>"}

"direct" means it meets the need as stated; "partial" means it covers part of the
need or needs significant work to fit. Judge from the fields given and nothing
else: do not use what you otherwise know about the repository. If none fit,
return the single line NONE. Be honest in your response.
```

A subagent that returns something other than JSON lines or `NONE` has not judged its chunk: rerun it once, and if it fails again, name the chunk as unjudged in the answer rather than treating it as empty.

### 4. Look up the details of what fit

Collect the titles every subagent returned and read their records from the full export:

```bash
jq '.notes[].frontmatter | select(.title | IN($ARGS.positional[]))
  | {title, url, summary, status, last_commit, archived}' <dir>/catalog.json --args <title> ...
```

Keep `--args` after the file: every argument after it is read as a title, the file included.

A title that comes back with no record was misquoted by its subagent; drop it rather than guess which repository it meant.

### 5. Answer

If nothing fit, say that the catalogue holds no repository for this need, give the restated need you searched for, and stop.

Otherwise list the matches, `direct` before `partial`, and within each by how well the reason meets the need.
For each, give:

- the `title`, linked to its `url`;
- the `summary`;
- the subagent's reason, checked against the summary — drop or demote one the summary does not bear out;
- a mark when the repository may not be maintained: `archived` when `archived` is true, and `dormant` with its `last_commit` when `status` is `dormant` (no push in over two years). Give `last_commit` for `slow` too, but without a warning, since a finished small library pushes rarely.

Where the matches all cover part of the need, lead with that: the catalogue holds nothing made for it, and say which part none of them covers.
The judges let general-purpose tools through as `partial` freely, so give in full only the few that come closest and name the rest in one line.

## Tool behaviour this skill relies on

Checked against k-boat's `kboat-note` on 2026-11-07, on a catalogue of 365 notes:

- `kboat-note list --type repo` returns `{anomalies, counts, notes: [{slug, path, frontmatter}]}`, and exits 1 when `Repos/` cannot be read;
- `domain` and a non-empty `topics` come back as a flow-list string such as `"[robotics, ml]"`, and an empty `topics` as an array;
- the compact export is about 220 KB, so four chunks of about 55 KB each.

Re-check these when the export or a subagent's output looks wrong, rather than trusting this section.

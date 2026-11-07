---
name: find-kboat-repo
description: "Find software for a need the user states in words — \"I want to do X\" — among the GitHub repositories they catalogued in K-Boat, and say plainly when the catalogue holds nothing that fits. Searches the catalogue only, never the web. Read-only; never writes to the vault."
disable-model-invocation: true
allowed-tools: Bash(${CLAUDE_SKILL_DIR}/scripts/export_catalog.sh)
---

# Find a repository in the K-Boat catalogue

K-Boat keeps one note per GitHub repository the user has come across, in the `Repos/` folder of their Obsidian vault.
This skill answers "is there something in my catalogue I could use for this?" from those notes, and from nothing else.
When nothing fits, say so and stop: an answer drawn from the web or from general knowledge would read as though the user had catalogued it.

Never write, edit or move a note; catalogue maintenance belongs to K-Boat's own skills.

## 1. Export the catalogue

Run `${CLAUDE_SKILL_DIR}/scripts/export_catalog.sh`.
It reads every repo note through K-Boat's `kboat-note list` and splits a compact copy into chunk files of 100 notes each, printing `{dir, notes, anomalies, chunks}`.

- A non-zero exit means the catalogue was not read, and nothing about its contents follows. Report the exit and what the script printed, and stop.
  - Exit 2 means `KBOAT_REPO`, the path to this machine's k-boat checkout, is not set; the "Setting up K-Boat" section of the dotfiles README says where it goes.
- A non-empty `anomalies` on a zero exit names notes that could not be read. Carry on, and name each beside the answer, since a repository the search did not see reads as one the user never catalogued.

Do not read `catalog.json` or `compact.jsonl` into this conversation; that is what the chunks are for.

## 2. Judge every chunk

Spawn one subagent per chunk, all in one message, each with `model: "haiku"`, and give each this prompt, with the need in the user's own terms:

```text
Read the file <chunk path>. Each line is one GitHub repository from a personal
catalogue, as JSON with title, role, domain, topics, description and summary
(summary is in Japanese).

The need: <the need>

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

Each judge reads its whole chunk rather than a keyword-filtered list, because `summary` is Japanese while `description` and `topics` are English, and a need put in words often shares no term with the repository that meets it.
A judge that returns neither JSON lines nor `NONE` has not judged its chunk: name the chunk as unjudged in the answer rather than treating it as empty.

## 3. Look up what fit

Read the records of every returned title from the full export:

```bash
jq '.notes[].frontmatter | select(.title | IN($ARGS.positional[]))
  | {title, url, summary, status, last_commit, archived}' <dir>/catalog.json --args <title> ...
```

Keep `--args` after the file, since every argument after it is read as a title.
A title with no record was misquoted by its judge; drop it rather than guess which repository it meant.

## 4. Answer

If nothing fit, say that the catalogue holds no repository for this need, and stop.

Otherwise list `direct` matches before `partial` ones, each with its `title` linked to its `url`, its `summary`, and the judge's reason, dropping a reason the summary does not bear out.
Mark `archived` when `archived` is true, and `dormant` with the `last_commit` when `status` is `dormant`.

When every match is `partial`, lead with that: the catalogue holds nothing made for this need, and say which part none of them covers.
Judges let general-purpose tools through as `partial` freely, so give in full only the few that come closest and name the rest in one line.

---
name: investigation-report
description: Write up a technical investigation or experiment as a self-contained report for a reader who did not work alongside you — a story-first overview linked to detailed records, shipped as one directory with the scripts and results behind every number.
disable-model-invocation: true
---

# Writing an investigation report

The reader did not follow the work.
They know neither the dead ends, nor the labels coined along the way, nor the files the work left behind.
A report that only someone who worked alongside you can read has failed that reader, however accurate it is.

Write its prose by [document-writing.md](../../rules/document-writing.md), and when the report is in Japanese, compose it with the `japanese-tech-writing` skill as well.

## Answer the reader's questions, not the history

- Build the report around the questions the requester wants answered; where the request names none, take them from the goals the investigation was set.
- Leave out the path taken: hypotheses that were dropped, the order things were tried, and course corrections. State what is established now and how it was shown.
- Cut asides that do not serve the main line, even true ones, such as which other releases never reach the code path at issue.
- When presenting more than one of something (two run configurations, two workarounds), say in a sentence what each is for, and nothing more.

## Make it self-contained

- A reader with no access to the rest of the workspace must be able to follow every sentence.
  - Link to no local file outside the report.
  - Use no name that only has meaning inside the work: host or pod names, queue and run labels, scratch file names.
  - Links to public sources, such as upstream source at a pinned revision, issues, and release notes, stay.
- Where the work touched something that is not to be shared, such as a private project or dataset, keep even hints of it out.
  - A result obtained only there is dropped, and the report says the matter was not tested instead.

## Ship it as one directory

- Put the body in `README.md`, the scripts that produced each number in `scripts/`, and their raw outputs and aggregates in `results/`.
- Refer to those files from the body by relative links, and zip the directory, so handing over the zip is the whole delivery.
- Make every number in the body recomputable from `results/` with a bundled script, writing an aggregation script wherever a number was computed ad hoc.

## Separate the story from the records

Give the body three top-level parts, in this order.

1. **Overview**: the story from cause to observed effect, in prose.
   - Walk it as a chain, in this order:
     - what happened
     - what caused it
     - how the cause produces the effect
     - the scope in which it occurs
     - what to do about it
   - Keep identifiers, source links, and numbers to what the story needs.
   - Close each paragraph that makes a claim with a link to the detailed section that backs it.
   - State in the overview what is not yet established, linking to where its status is recorded.
2. **Detailed records**: one section per overview claim, holding the conditions, measurements, tables, and source links.
   - Open with the conditions every measurement shares, including criteria defined for the analysis (what counts as a spike, say).
   - Make every section reachable by a link from the overview.
3. **Bundled files and reproduction**: what each file is and which section it backs, how the environments were built, and the commands that rerun the measurements and aggregations.

## Carry hypotheses with their status

- Do not stop at observations: where the mechanism is not established, state hypotheses.
- For each hypothesis, give the observations that support it, those that do not fit or cannot yet tell it apart from an alternative, and whether and how it was verified.
- An explanation that is appealing but has no evidence may appear as background, marked as untested, and stays out of the conclusion.
- State a limitation as the question it leaves open, not as a bare fact about the measurement.
  - NG: "the in-training comparison ran for only 22 steps."
  - OK: "no record shows whether the error grows around the spikes, since the in-training comparison covers only the first 22 steps."

## Place terms and links where they are used

- Write no up-front glossary. Define a term at its first use, and only one that is both uncommon and central to the report.
- Do not put a link in parentheses after a statement; say in the sentence what the linked thing is.
- Where a passage would carry many source links, make it a bulleted list with one fact per item, and put the link on the identifier that is the evidence.

## Check before handing over

- Every number in the body matches the bundled output it came from.
- Every section link resolves to an existing heading, and every relative link to an existing file.
- The aggregation commands in the reproduction part, rerun on a copy, regenerate the bundled aggregates.
- A search of the whole directory finds none of the following:
  - tokens
  - personal names
  - IP addresses
  - names of anything kept out under [Make it self-contained](#make-it-self-contained)

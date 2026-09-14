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

The reader comes to learn what is true and how it was shown.
Anything else spends their attention without bringing them closer to that.

- Build the report around the questions the requester wants answered; where the request names none, take them from the goals the investigation was set.
- Leave out the path taken, such as dropped hypotheses, the order things were tried, and course corrections, since a reader who meets a dropped hypothesis has to work out for themselves that it no longer holds. State what is established now and how it was shown.
- Cut asides that do not serve the main line, even true ones, such as which other releases never reach the code path at issue. The reader looks for how each one bears on the conclusion and finds nothing.
- When presenting more than one of something (two run configurations, two workarounds), say in a sentence what each is for, and nothing more. Without that sentence the reader cannot tell which result backs which claim, and a longer account is weight the story does not need.

## Make it self-contained

The report travels without its workspace.
A local link the reader cannot open, or a label they never learned, stops them at that sentence.

- Write every sentence so that a reader with no access to the rest of the workspace can follow it.
  - Link to no local file outside the report.
  - Use no name that only has meaning inside the work: host or pod names, queue and run labels, scratch file names.
  - Keep links to public sources, such as upstream source at a pinned revision, issues, and release notes, since the reader can open those and they carry the evidence.
- Where the work touched something that is not to be shared, such as a private project or dataset, keep even hints of it out, because a report gets passed on beyond the reader it was written for.
  - Drop a result obtained only there, and say instead that the matter was not tested. Citing a result the reader can never inspect asks them to take it on trust.

## Ship it as one directory

Evidence scattered across files is hard to take in and hard to check.
One directory lets the reader reach everything from the body and verify a number without asking you.

- Put the body in `README.md`, the scripts that produced each number in `scripts/`, and their raw outputs and aggregates in `results/`.
- Refer to those files from the body by relative links, and zip the directory, so handing over the zip is the whole delivery.
- Make every number in the body recomputable from `results/` with a bundled script, writing an aggregation script wherever a number was computed ad hoc. Recomputing is also what catches a number carried over wrong from the working notes.

## Separate the story from the records

Measurements, identifiers, and source links met before the reader knows what they establish are a load only someone who did the work can carry.
Let the overview tell the causal story first, and let the reader go down into a record only for the claim they want to check.

Give the body three top-level parts, in this order.

1. **Overview**: the story from cause to observed effect, in prose.
   - Walk it as a chain, in this order:
     - what happened
     - what caused it
     - how the cause produces the effect
     - the scope in which it occurs
     - what to do about it
   - Keep identifiers, source links, and numbers to what the story needs, since each one it does not use is a detour.
   - Close each paragraph that makes a claim with a link to the detailed section that backs it, so a claim is never more than one step from its evidence.
   - State in the overview what is not yet established, linking to where its status is recorded. An overview that leaves it out reads as if everything in it were settled.
2. **Detailed records**: one section per overview claim, holding the conditions, measurements, tables, and source links.
   - Open with the conditions every measurement shares, including criteria defined for the analysis (what counts as a spike, say), so the sections after it need not repeat them.
   - Make every section reachable by a link from the overview. A section no claim points to is either an aside or a claim the overview is missing.
3. **Bundled files and reproduction**: what each file is and which section it backs, how the environments were built, and the commands that rerun the measurements and aggregations.

## Carry hypotheses with their status

Observations alone hand the reader the job of explaining them, which is the job the report was written to do.
Stating each hypothesis's status is what keeps it from being read as a finding.

- Where the mechanism is not established, state hypotheses rather than stopping at observations.
- For each hypothesis, give the observations that support it, those that do not fit or cannot yet tell it apart from an alternative, and whether and how it was verified.
- An explanation that is appealing but has no evidence may appear as background, marked as untested, and stays out of the conclusion. Readers tend to reach for such an explanation on their own, so naming it tells them it was considered without letting it pass as a finding.
- State a limitation as the question it leaves open, not as a bare fact about the measurement, since the fact alone does not tell the reader why it matters.
  - NG: "the in-training comparison ran for only 22 steps."
  - OK: "no record shows whether the error grows around the spikes, since the in-training comparison covers only the first 22 steps."

## Place terms and links where they are used

A term or link placed away from where it is needed makes the reader hold it in mind, or open it, before they know why it matters.

- Write no up-front glossary. Define a term at its first use, and only one that is both uncommon and central to the report; a common term defined anyway is noise, and one defined far from its use is forgotten by then.
- Do not put a link in parentheses after a statement; say in the sentence what the linked thing is, since a bare link leaves the reader to open it to find out.
- Where a passage would carry many source links, make it a bulleted list with one fact per item, and put the link on the identifier that is the evidence. Prose that carries many links grows phrases that exist only to hold a link.

## Check before handing over

Once the report is handed over, you are not there to answer for it.
Each check catches a mistake the reader could not detect, or a leak no one could undo.

- Every number in the body matches the bundled output it came from.
- Every section link resolves to an existing heading, and every relative link to an existing file.
- The aggregation commands in the reproduction part, rerun on a copy, regenerate the bundled aggregates.
- A search of the whole directory finds none of the following:
  - tokens
  - personal names
  - IP addresses
  - names of anything kept out under [Make it self-contained](#make-it-self-contained)

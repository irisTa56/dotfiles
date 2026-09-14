---
name: investigation-report
description: Write up a technical investigation or experiment as a report that a reader who did not work alongside you can understand — one story told first, with the detailed records kept apart and linked from it.
disable-model-invocation: true
---

# Writing an investigation report

The reader did not follow the work.
A report that only someone who worked alongside you can understand has failed that reader, however accurate it is.

Write its prose by [document-writing.md](../../rules/document-writing.md), and when the report is in Japanese, compose it with the `japanese-tech-writing` skill as well.

## Tell one story first

Details mean little to a reader who does not yet know what they establish.
So the report opens with a story the reader can follow, and the details come after it.

- Build the story around the questions the requester wants answered.
- Give the story one message, running from those questions to the conclusion.
- Do not flatten the facts to fit that message; they are rarely simple, and where their complexity is itself informative, show it and say how it bears on the message.
- Keep the story abstract, bringing in identifiers, numbers, and source links only where the message needs them.
- Close each paragraph that makes a claim with a link to the record that backs it.

## Keep the records apart from the story

Records serve a different need: checking a claim, or doing the work again.
Writing them is not the problem; a report whose records stand alone, or come before the story, is.

- Place the records after the story, one section per claim, each reachable by a link from the story.
- Record the course of the work wherever it helps someone repeat it.
- Record a pitfall or an alternative set aside when it bears on the main line or on the reader, and finding it out takes time or could lead to rework.
- Keep a result that runs against the story in the records, and point to it from the story rather than leaving it out.

## Separate what is established from what is not

A reader cannot tell a finding from a guess unless the report tells them.

- Where the mechanism is not established, state hypotheses rather than stopping at observations.
- For each hypothesis, give what supports it, what does not fit, and whether it has been verified.
- Say so when a hypothesis was formed after seeing the observations it explains.
- Give the number of runs behind each result, and say so where variation across runs was not measured.
- State a limitation as the question it leaves open, not as a bare fact about the measurement.

## Make it self-contained

The report reaches readers without the workspace it was written in.

- Link to no local file outside the report.
- Use no label that only has meaning inside the work.
- Define each term the reader may not know at its first use.
- Add a glossary only when it is limited to specialized terms central to the story.

## Keep out what is not to be shared

A report gets passed on beyond the reader it was written for.

- Where the work touched something not to be shared, such as a private project or dataset, keep even hints of it out of the report and the bundled files.
- Drop a result obtained only there, and where the point matters, say the report has no result on it rather than that it was never tried.

## Ship the evidence with the report

This applies when the investigation produced scripts or raw data.
With them in hand, the reader can verify a number without asking you.

- Put the report and those files in one directory, linked by relative paths, so that the directory, or a zip of it, is the whole delivery.
- Make every number in the report recomputable from the bundled files.
- Close the report with what each file is, which record it backs, and how to rerun it.

## Check before handing over

- Read the story alone, and confirm it makes sense without the records.
- Confirm every number matches the output it came from.
- Confirm every section link and relative link resolves.
- Search everything handed over, scripts and logs included, for anything that should not leave with it, such as:
  - tokens and other credentials
  - personal names and user names
  - absolute local paths
  - host names, IP addresses, and internal URLs
  - names of anything kept out

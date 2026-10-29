---
name: pr-description
description: Principles for writing or revising a pull request description so it serves the human reviewer. Use when drafting a PR body, rewriting one, or critiquing a change summary before submitting. Repository-agnostic and template-free — it governs what to say and what to cut, not a fixed section layout. When the PR is written in Japanese, compose with the japanese-tech-writing skill.
---

# Writing a PR description

A PR description is written first for the human reviewer who must judge the change now, not as an exhaustive archive.
Every line must help them decide whether the change is correct and where to look.
Cut anything that does not.

Apply these when drafting, rewriting, or critiquing a PR body.
They are about substance, not a fixed section layout — adapt to whatever template (or no template) the repository uses.
Give the body an outline in headings or bullets: unbroken prose makes the reader read the whole body to find the shape of the change.
Take that outline from what the change actually has to say.

## Honor the repository's required conventions first

- Before drafting, find and read the repository's PR conventions: `.github/PULL_REQUEST_TEMPLATE.md` (and any `PULL_REQUEST_TEMPLATE/` directory), `CONTRIBUTING`, and the like.
- The principles below govern substance *within* that structure; they never license skipping a mandatory section, checklist item, or sign-off. Keep and fill every required field, even one you judge low-value.
- The pruning advice that follows (earn every section, cut non-changes) applies only to content you control and to sections the template leaves optional — never to fields the repository mandates.

## Be self-contained at the overview level

- Make the title say what the PR does, with no identifier from the plan the work followed (a session number, a task or step id), which a reader who has not seen the plan cannot decode.
- Convey the change at an overview level in the body itself. The reviewer should grasp what changed and why without opening the linked ticket, issue, or prior PR.
- Open the body with that overview: what the change does, stated as its effect, and why it is needed.
  - Mechanism, the exact scope of the old behavior, and alternatives come after it; a reviewer who stops after the opening should already know what they are judging.
- Deferring depth to the link is fine, and often preferable: background, full specifications, and exhaustive context belong behind the ticket link rather than inlined in the body.
- What must not depend on the link is the gist.
  - NG (gist outsourced to the link): a rationale that amounts to "updated per the policy in #1234".
  - OK: state the approach in the body (e.g. "raise each direct dependency's lower bound and refresh the lock with `uv lock --upgrade`"), and link the ticket for the details.

## List changes, not non-changes

- List only what actually changed. A decision *not* to change something is not a change, and listing it alongside real changes reads wrong.
  - NG: a bullet "kept `ipykernel` pinned `<7.0.0`" among the change bullets.
- Drop caveats that only become a concern if you raise them. If something needs no action from the reviewer and introduces no risk, leaving it out is clearer than explaining it.
  - Example of what to omit: internal resolution details nobody would otherwise question (e.g. "upstream-pinned transitive deps resolved via their parents").
- If a non-change genuinely needs recording, put it where it belongs (an inline code comment at the pin, say), not in the PR's change list.

## Frame an exception against the rule it breaks

- When one change is an exception to an otherwise uniform or automated operation, state that relationship, or its presence in the list looks arbitrary.
  - OK: "The only change not left to `uv lock --upgrade` is swapping the server's `httpx` test backend for `httpx2`, because …".

## Don't restate the diff

- Do not enumerate what the diff already shows: per-package version bumps, file lists, line moves. Summarize one level up and let the reviewer read specifics from the diff.
  - NG: a bullet list of "fastapi 0.135 → 0.138, uvicorn 0.41 → 0.49, …".
  - OK: "bumped the direct dependencies' lower bounds and refreshed the full lock".

## Earn every section and claim

- Do not fill sections mechanically. If a section (design rationale, review focus, …) holds nothing useful to a reviewer, write the template's "none" marker when the section is required, and omit only sections the template leaves optional. An honest "none" beats a paragraph of filler.
- Merge sections when the content sits better together. Often the one risk worth flagging belongs in the same place as the verification that clears it, rather than split across "review focus" and "notes".
- Leave out what records the work rather than the change: run results, cost or time estimates, and the history of how the work progressed (such as the order things were tried in, or the sessions it took).
  - Those belong where the project tracks the work, such as a note on the ticket; the body is what the reviewer judges the change from.
  - A result that measures the effect the change claims, such as a latency drop, is evidence, not a record: where the reviewer can reproduce it, such as a benchmark run by a command in the repository, state it with its figures and how to rerun it, unless that would expose context that must stay private; where they cannot, [State verification as a judgment](#state-verification-as-a-judgment-not-a-metrics-dump) applies.
  - An alternative the change did not take stays as design rationale, written as a property of the design ("polling would delay each update by up to one interval") rather than as something tried and reverted.
- When a review focus will be read by an AI reviewer that has no context beyond the PR, such as an automated PR reviewer, write it for that reader.
  - Ask only what the diff, or information anyone can look up, can confirm. A question whose answer rests on something neither holds, such as production traffic or a measured outcome, gets a guess back.
    - Check before opening the PR whatever can be checked before merge. State in the verification judgment only what cannot be checked until after merge, such as production behavior, with how it will be checked then, so the human reviewer still sees it.
  - Explain each term at its first use, such as an environment variable or a metric name, since the reviewer has not seen the conversation that coined or used it.

## State verification as a judgment, not a metrics dump

- Make a claim the reviewer can accept or challenge: "this includes major updates, but it passed <these checks>, so I judge the migration safe."
- Do not paste exhaustive evidence — test counts, timings, HTTP statuses, coverage percentages. Decorative precision does not aid review and buries the claim.
- A result the reviewer cannot reproduce, such as a measurement on a machine or dataset they cannot access, supports nothing on its own, with its figures or as a bare verdict. Give the claim behind it a route the reviewer can follow: argue it from the change's logic, rest it on general knowledge, or describe the system as far as it can be written and state the premises the result depends on before the result.

## Keep each claim as narrow as the truth

- A summary or risk claim must hold exactly as stated. Before writing "the only X is Y", check whether transitive or indirect cases break it, and scope the sentence to what is actually true.
  - NG: "the only major update is `starlette`" when transitive majors (`pyarrow`, `datafusion`) also moved.
  - OK: "the only major update affecting the server is `starlette`" — true because those transitive majors live in a different component's closure.

## Match the project's language and register

- Write in the language and register the repository and its reviewers use; infer the convention from recent merged PRs rather than guessing.
- For a Japanese description, default to `ですます調` unless the repo's convention differs, and follow the `japanese-tech-writing` rules (one sentence per line, no LLM filler, precise terms).

## Final pass

Re-read the draft and check:

- Does the draft satisfy every section and checklist item the repository requires?
- Does the title say what the PR does, with no identifier from the plan the work followed?
- Does the opening alone tell a reviewer what the change does and why?
- Can a reviewer see the shape of the change without reading the whole body?
- Would a reviewer who cannot open the linked ticket still understand the change?
- Does every change bullet describe an actual change, with no non-changes, self-inflicted caveats, or internal details nobody would otherwise question?
- Is each exception framed against the rule it breaks, so it does not look arbitrary?
- Did I restate anything already obvious from the diff?
- Is any summary claim broader than what is strictly true?
- If I edited the body after it was last checked against the code, shortening included, did I check each claim I changed against the code again?
- Does each section earn its place, or is it filled for form's sake?
- Did any record of the work (a run result that does not measure the change's claimed effect, a cost or time figure, progress history) stay in the body, and can the reviewer rerun or follow each result that does?
- If an AI reviewer will read the review focus, can the diff or public information answer each question it asks?
- If so, is each term the review focus uses explained at its first use?
- Is verification phrased as a judgment ("safe because …"), not a pile of metrics?

---
name: pr-description
description: Principles for writing or revising a pull request description so it serves the human reviewer. Use when drafting a PR body, rewriting one, or critiquing a change summary before submitting. Repository-agnostic and template-free — it governs what to say and what to cut, not section layout. When the PR is written in Japanese, compose with the japanese-tech-writing skill.
---

# Writing a PR description

A PR description is written first for the human reviewer who must judge the change now, not as an exhaustive archive. Every line must help them decide whether the change is correct and where to look. Cut anything that does not.

Apply these when drafting, rewriting, or critiquing a PR body. They are about substance, not a fixed section layout — adapt to whatever template (or no template) the repository uses.

## Honor the repository's required conventions first

- Before drafting, find and read the repository's PR conventions: `.github/PULL_REQUEST_TEMPLATE.md` (and any `PULL_REQUEST_TEMPLATE/` directory), `CONTRIBUTING`, and the like.
- The principles below govern substance *within* that structure; they never license skipping a mandatory section, checklist item, or sign-off. Keep and fill every required field, even one you judge low-value.
- The pruning advice that follows (earn every section, cut non-changes) applies only to content you control and to sections the template leaves optional — never to fields the repository mandates.

## Be self-contained at the overview level

- Convey the change at an overview level in the body itself. The reviewer should grasp what changed and why without opening the linked ticket, issue, or prior PR.
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

## State verification as a judgment, not a metrics dump

- Make a claim the reviewer can accept or challenge: "this includes major updates, but it passed <these checks>, so I judge the migration safe."
- Do not paste exhaustive evidence — test counts, timings, HTTP statuses, coverage percentages. Decorative precision does not aid review and buries the claim.

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
- Would a reviewer who cannot open the linked ticket still understand the change?
- Does every change bullet describe an actual change, with no non-changes or self-inflicted caveats?
- Is each exception framed against the rule it breaks, so it does not look arbitrary?
- Did I restate anything already obvious from the diff?
- Is any summary claim broader than what is strictly true?
- Does each section earn its place, or is it filled for form's sake?
- Is verification phrased as a judgment ("safe because …"), not a pile of metrics?

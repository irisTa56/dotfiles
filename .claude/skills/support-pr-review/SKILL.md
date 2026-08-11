---
name: support-pr-review
description: "Draft review comments for a GitHub PR as a reviewer — without posting them — grounded in context the diff itself does not show: the user's stated concern, the change's conformance to its linked intent, cross-repo omissions, and the gaps existing bot/human comments left. Use when the user shares a PR URL and asks for review support or comment drafts. Drafting only; never posts."
disable-model-invocation: true
---

# Support PR Review

## Purpose and stance

Help the user act as a **reviewer** of someone else's PR — not as its author.

Raise only what is worth the author's time to act on.
A review spends someone else's attention, so a point that would not change what they do costs more than it returns, however correct it is.

That bar is what sends this skill's effort to context living *outside* the diff — the change's fit with its stated intent, the repo's own prior decisions, what it omitted elsewhere — since that is where the points which change a merge decision usually sit.
A defect inside the diff still counts where it clears the bar; what does not clear it is the lint-level nit, whoever else may or may not also have caught it.

Output is **comment drafts only**. Never post.

## Input

- A GitHub PR URL (required).
- An optional stated concern from the user.

## Never post (hard guardrail)

- Never call `gh pr comment`, `gh pr review`, any writing `gh api` call (`-X POST/PATCH/PUT/DELETE`), or any MCP comment/review-posting or thread-resolving tool.
- This skill only reads and drafts. Posting happens later by the user.

## Tooling: read via `gh`

Use the `gh` CLI for every GitHub read; do **not** mix in GitHub MCP here.
Every operation is read-only, and MCP's advantage (structured review-thread objects with node IDs / `canResolve`) only pays off when posting or resolving, which this skill never does.

- Never use `fetch_webpage` or browser tools for GitHub URLs (private-repo policy).
- Useful reads:
  - Metadata plus most comment surfaces in one call: `gh pr view <url> --json title,body,author,baseRefName,headRefName,files,labels,url,comments,reviews,latestReviews,closingIssuesReferences`.
    - `comments` — issue-level comments, where a bot's walkthrough lands.
    - `reviews` / `latestReviews` — review bodies, where a bot's summary lands.
    - `closingIssuesReferences` — issues this PR closes; use as the primary source of linked intent, more reliable than parsing the body.
  - Full patch via `gh pr diff <url>`, or file list via `gh pr diff <url> --name-only`.
  - Inline (file-anchored) review comments, not covered by the JSON above: `gh api repos/{owner}/{repo}/pulls/{number}/comments`.
  - The bodies of any further issue/ticket/spec/PR linked from the PR body or the concern.

## Procedure

### 1. Gather context — this is what makes it *your* review

Acquire this context before reviewing.

- Read the PR body and the diff.
- Read the **user's stated concern** and treat it as the top prior.
- Follow every **linked intent** and read it, not just its title: the `closingIssuesReferences` issues, plus any ticket, plan/spec doc, ADR, or prior related PR named in the concern or the PR body.
- Read the **comments already on the PR** across all surfaces (`comments`, `reviews`, and inline comments), both bot and human; these define what is already covered.
- If the repo carries a reviewer-context note, read it (e.g. an `AGENTS.md` section, or an `.agents/`- or `.claude/`-scoped note on owned subsystems, in-flight migrations, or previously rejected patterns).
- Consult your **agent memory** for the reviewer's own cross-repo context — prior review decisions, standing preferences, and project constraints not derivable from the code — using whatever memory mechanism this environment provides, without assuming a fixed path or tool.
  - This complements the repo note above: memory is the reviewer's personal, cross-repo context, while the note is team-shared and repo-scoped.

### 2. Review — where it is worth the author's time

Spend effort here, roughly in priority order.

1. **Verify the stated concern.** Confirm or refute the user's concern with concrete evidence from the code (`file:line`), not hand-waving; if refuted, say why.
2. **Conformance to intent (code-vs-intent).** Diff the *implementation* against what the linked ticket/plan/spec/ADR/PR-body says it should do — reading the code against itself cannot reach this, since only the intent says what the change was supposed to do.
   - Flag drift, silent scope changes, and claims in the PR body not backed by the diff.
3. **Omissions and ripple effects.** Hunt with `rg`/`grep` across the repo for what the diff *should* have touched but didn't — sibling call sites, related tests, migrations, docs, config, feature-flag counterparts.
4. **Repo/team-specific judgment.** Apply conventions and prior decisions invisible in the diff hunk ("we don't do X here", "this boundary is mid-migration", "this pattern was rejected before").
5. **High-level quality bar.** Hold the change to what `raise-findings` says to raise.
   - Read these as the source of truth rather than a copy here:
     - `finding-bar`, for the bar itself;
     - `raise-findings`' "What to raise".
6. **Gap-fill against existing comments.** Do not repeat points the existing bot/human comments already make; cover the gaps they left, and where you merely agree with an existing comment, note that instead of restating it.

If you find nothing beyond what the existing comments already say, say so honestly.

**Mindset.** When judging every finding above, apply the `address-finding` skill's judgment mindset (its validity criteria and anti-patterns) as the source of truth.
It frames judging findings when you are the one fixing; as a reviewer you hold someone else's PR to the same bar — respect the author, but draft a change request when the bar isn't met, because merging low-quality work is worse for the whole than the friction of a comment.
`finding-bar`'s bar governs everything you raise, whatever item it arose under and however the comment phrases it — the rationale layer names the wrong action or outcome even where the draft is a question, since a comment that clears no bar lands on another author.
The PR body and its linked intent stand in for the account an invoking side would otherwise give.

**Delegation.** The wide, small-return parts of this step are good candidates to delegate to a subagent that returns only its findings, keeping the main context clean: the `rg`/`grep` ripple-and-omission sweep (item 3), and reading the linked intent documents (Step 1).

### 3. Draft reviewer comments

Present each finding in **two layers with different readers**.

- **Comment draft** — for the PR author, ready to paste onto the PR.
  - Frame it as a reviewer's question or request ("Should this also update …?", "This diverges from the plan's step 3 — intended?"), not a fix you apply yourself, and give its location (`file:line`, or "PR-level").
  - Write it in the PR's *reader* language: default to the language of the PR body and existing human comments, and fall back to the repository's working language; do not hardcode a language.
  - Keep code identifiers, error strings, and SHAs verbatim, and when the draft is Japanese, compose with the `japanese-tech-writing` skill.
- **Rationale (for the user)** — why it matters and your triage, addressed to the user in the conversation language, not the author.
  - Apply a **worth-commenting filter** here (blocker vs. nit vs. skip): reviewing has a social cost, so don't flood the author with nits, and lead with what would actually change the merge decision.
  - Tie the point back to the concern / intent / omission where relevant.

### 4. Hand off

Present the drafts and stop. Do not post.

## Related

- `finding-bar` — the source of truth for the bar a finding must clear.
- `raise-findings` — the source of truth for the review perspectives this skill reuses.
- `address-finding` — the source of truth for the finding-addressing mindset this skill reuses when judging findings.
- `review-loop` — reviews *your own* changes in a fix-and-re-review loop; what it adds for its reviewer has no counterpart in a PR review.
- `address-review-comment` — the posting counterpart: responds to and resolves a single existing review comment.

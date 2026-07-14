---
name: support-pr-review
description: "Draft review comments for a GitHub PR as a reviewer — without posting them — grounded in context a generic diff bot cannot see: the user's stated concern, the change's conformance to its linked intent, cross-repo omissions, and the gaps existing bot/human comments left. Use when the user shares a PR URL and asks for review support or comment drafts. Drafting only; never posts."
disable-model-invocation: true
---

# Support PR Review

## Purpose and stance

Help the user act as a **reviewer** of someone else's PR — not as its author.
Generic review bots (Copilot, CodeRabbit, the `code-review-expert` skill) already scan the diff for lint-level nits, generic SOLID/security patterns, and in-diff correctness.
Re-running that adds nothing but duplication.
This skill deliberately spends its effort on what those bots structurally **cannot** do: review the change against context that lives *outside* the diff.

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
One tool, no branching.

- Never use `fetch_webpage` or browser tools for GitHub URLs (private-repo policy).
- Useful reads:
  - Metadata plus most comment surfaces in one call: `gh pr view <url> --json title,body,author,baseRefName,headRefName,files,labels,url,comments,reviews,latestReviews,closingIssuesReferences`.
    - `comments` — issue-level comments, where a CodeRabbit walkthrough lands.
    - `reviews` / `latestReviews` — review bodies, where a Copilot summary lands.
    - `closingIssuesReferences` — issues this PR closes; use as the primary source of linked intent, more reliable than parsing the body.
  - Full patch via `gh pr diff <url>`, or file list via `gh pr diff <url> --name-only`.
  - Inline (file-anchored) review comments, not covered by the JSON above: `gh api repos/{owner}/{repo}/pulls/{number}/comments`.
  - The bodies of any further issue/ticket/spec/PR linked from the PR body or the concern.

## Procedure

### 1. Gather context — this is what makes it *your* review

The differentiator exists only if the agent actually holds the user's context.
Acquire it before reviewing.

- Read the PR body and the diff.
- Read the **user's stated concern** and treat it as the top prior.
- Follow every **linked intent** and read it, not just its title: the `closingIssuesReferences` issues, plus any ticket, plan/spec doc, ADR, or prior related PR named in the concern or the PR body.
- Read the **comments already on the PR** across all surfaces (`comments`, `reviews`, and inline comments), both bot and human; these define what is already covered.
- If the repo carries a reviewer-context note, read it (e.g. an `AGENTS.md` section, or an `.agents/`- or `.claude/`-scoped note on owned subsystems, in-flight migrations, or previously rejected patterns).
- Consult your **agent memory** for the reviewer's own cross-repo context — prior review decisions, standing preferences, and project constraints not derivable from the code — using whatever memory mechanism this environment provides, without assuming a fixed path or tool.
  - This complements the repo note above: memory is the reviewer's personal, cross-repo context, while the note is team-shared and repo-scoped.

### 2. Review — only where you beat the bots

Spend effort here, roughly in priority order, and skip anything a diff bot already handles well.

1. **Verify the stated concern.** Confirm or refute the user's concern with concrete evidence from the code (`file:line`), not hand-waving; if refuted, say why.
2. **Conformance to intent (code-vs-intent).** Diff the *implementation* against what the linked ticket/plan/spec/ADR/PR-body says it should do — bots check code-vs-code, you check whether the change does what was actually agreed.
   - Flag drift, silent scope changes, and claims in the PR body not backed by the diff.
3. **Omissions and ripple effects.** Hunt with `rg`/`grep` across the repo for what the diff *should* have touched but didn't — sibling call sites, related tests, migrations, docs, config, feature-flag counterparts.
   - This is the highest-value, bot-weakest area.
4. **Repo/team-specific judgment.** Apply conventions and prior decisions invisible in the diff hunk ("we don't do X here", "this boundary is mid-migration", "this pattern was rejected before").
5. **High-level quality bar.** Beyond in-diff correctness, hold the change to the review perspectives that the `review-loop` skill defines (design integration, test quality, minimal declarative config).
   - Read that skill's "Perspectives for the Review" section as the source of truth rather than a copy here.
6. **Gap-fill against existing comments.** Do not repeat points the existing bot/human comments already make; cover the gaps they left, and where you merely agree with an existing comment, note that instead of restating it.

If you find nothing beyond what the bots already said, say so honestly.

**Mindset.** When judging every finding above, apply the `address-finding` skill's judgment mindset (its validity criteria and anti-patterns) as the source of truth.
It frames judging findings when you are the one fixing; as a reviewer you hold someone else's PR to the same bar — respect the author, but draft a change request when the bar isn't met, because merging low-quality work is worse for the whole than the friction of a comment.

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

- Present the drafts and stop. Do not post.
- If the user then wants to post, that is the user's action — this skill ends at drafts.

## Related

- `review-loop` — the source of truth for the review perspectives this skill reuses; it reviews *your own* changes in a fix-and-re-review loop.
- `address-finding` — the source of truth for the finding-addressing mindset this skill reuses when judging findings.
- `address-review-comment` — the posting counterpart: responds to and resolves a single existing review comment.
- `code-review-expert` — generic diff review; intentionally the layer this skill does not duplicate.

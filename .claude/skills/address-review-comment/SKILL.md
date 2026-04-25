---
name: address-review-comment
description: Address a single GitHub PR review comment end-to-end. Use when the user provides a link to a PR review comment. Reads the comment via MCP/gh CLI (never fetch_webpage), evaluates validity, applies fixes, commits, drafts a reply, and posts it.
---

# Address Review Comment

Handle a single GitHub PR review comment end-to-end: read → evaluate → fix → commit → reply.

## Input

A GitHub PR review comment URL, e.g.:

- `https://github.com/{owner}/{repo}/pull/{number}#discussion_r{comment_id}`
- `https://github.com/{owner}/{repo}/pull/{number}/files#r{comment_id}`

## Procedure

### 1. Parse the URL

Extract from the comment link:

- `owner`, `repo`, PR `number`
- `comment_id` from the URL fragment (`#discussion_r{id}` or `#r{id}`)

### 2. Read the Comment

**NEVER use `fetch_webpage` or browser tools for GitHub URLs.**

Use `gh` CLI to fetch the specific review comment:

```bash
gh api repos/{owner}/{repo}/pulls/comments/{comment_id}
```

From the response, extract:

- `body` — the reviewer's comment text
- `id` / `node_id` — REST numeric ID and GraphQL node ID of the comment
- `user.login` / `user.type` — reviewer identity and actor type
- `path` — the file the comment refers to
- `line` / `original_line` — the line number
- `diff_hunk` — the surrounding diff context
- `in_reply_to_id` — if this is part of a thread, the parent comment ID

After extraction, set a tentative reply target type without interrupting the user:

- Prefer `ai-bot` when any of the following is true:
  - `user.type == "Bot"`
  - `user.login` matches common bot naming patterns (e.g., ends with `[bot]`)
- Otherwise, set `human`

Treat this as provisional and finalize at Step 7.

If the comment is part of a thread, also fetch the full thread:

```bash
gh api repos/{owner}/{repo}/pulls/{number}/comments \
  | jq '[.[] | select(.in_reply_to_id == {parent_id} or .id == {parent_id})]'
```

### 3. Understand the Context

- Read the file and lines the comment refers to
- Understand the reviewer's suggestion or concern
- Look at surrounding code for broader context if needed

### 4. Evaluate Validity

Take the reviewer's suggestion seriously, but do NOT blindly accept:

- **Technically correct?** Does the suggestion fix a real bug, improve correctness, or address a genuine concern?
- **Improves quality?** Does it improve readability, maintainability, or performance?
- **Trade-offs?** Are there considerations the reviewer may not have seen?
- **Scope appropriate?** Is the suggestion proportional, or over-engineering?
- **Step back once**: Is there a more fundamental fix that removes repeated manual work or prevents the class of issue (e.g., automation/script/abstraction instead of ad-hoc procedural edits)?

Decide on one of:

| Verdict | Action |
|---------|--------|
| Valid | Apply the fix as suggested (or a better variant that addresses the same concern) |
| Valid but not fundamental | Apply a more structural fix and explain why it is safer/lower future cost |
| Partially valid | Propose a balanced fix and explain reasoning |
| Not valid | Prepare a respectful explanation of why |

### 5. Apply Fix (if needed)

- Make the **minimal, targeted** change that addresses the comment
- Do NOT modify code outside the scope of the comment
- Do NOT add unrelated improvements
- If the chosen verdict requires code changes, complete all file edits in this step before moving to Step 7

### 6. Draft Reply

Compose a reply draft early, before final confirmation:

- Write in the **same language as the reviewer's comment**
- Select tone by reply target type:
  - `human`: concise, professional, and courteous (brief appreciation is allowed)
  - `ai-bot`: concise, professional, and factual (avoid unnecessary pleasantries)
- If a fix is planned or made: acknowledge and briefly describe the change, then include a placeholder on a new line (e.g., `\n\nFixed in <commit_sha>.`)
- If no fix will be made: explain the reasoning respectfully

### 7. Confirm Fix with User

Present the diff and reasoning to the user as a **single final execution checkpoint**.

Checklist:

- Precondition: if code changes are required, confirm the working tree already contains the intended edits; otherwise, return to Step 5 before asking for approval
- Show what was changed and why
- If no change was made, explain why the suggestion was declined or deferred
- Show inferred reply target type (`human` or `ai-bot`) and finalize it here together with commit/push confirmation
- Ask for override only when the user disagrees or the inference confidence is low
- Show the Step 6 draft reply that will be posted after commit/push

Wait for user approval, then execute the remaining steps in order without another confirmation.

### 8. Commit & Push

**Skip this step if no code change was made** — proceed directly to Step 9.

Once approved, commit with a **concise English message** in conventional commit format and push:

```bash
git add <changed-files>
git commit -m "<type>: <concise description>"
git push
```

- Choose the appropriate type: `fix`, `refactor`, `style`, `docs`, `perf`, etc.
- Keep the description under 72 characters
- Stage only the files related to this review comment
- Push to the current branch so the commit is visible on the PR

### 9. Post Reply

Post the approved reply using `mcp_github_add_reply_to_pull_request_comment`:

- `owner`, `repo` — from the parsed URL
- `commentId` — the comment ID from step 1
- `body` — the approved reply text (replace `<commit_sha>` with actual SHA after Step 8 when a fix was made)

After posting:

- If reply target type is `human`: do not resolve the thread
- If reply target type is `ai-bot`: resolve the thread

Resolve flow for `ai-bot`:

- Call `github-pull-request_currentActivePullRequest` and read `reviewThreads`
- Treat `commentId` as a REST API numeric ID (not a review thread ID)
- Map `commentId` to a thread by checking nested thread comments:
  - First try direct numeric match (`comment.id == commentId` or `comment.databaseId == commentId` if available)
  - If only node IDs are available, use the `node_id` captured in Step 2 to match thread comments
- If found and `canResolve` is `true`, call `github-pull-request_resolveReviewThread` with that thread `id`
- If not found or `canResolve` is `false`, skip resolving and only keep the posted reply

## Important Rules

1. **GitHub Private Repo Policy**: NEVER use `fetch_webpage` or browser tools for GitHub URLs. Always use MCP tools or `gh` CLI.
2. **One comment at a time**: Handle a single comment per invocation.
3. **Single final checkpoint**: Ask once at Step 7, then run commit/push and posting in sequence.
4. **Commit language**: Always English, conventional commit format.
5. **Reply language**: Always match the reviewer's comment language.
6. **Commit hash in reply**: Always include the commit SHA when a fix was made.
7. **Reply target type**: Infer after reading comment metadata, then finalize at Step 7 (single checkpoint).
8. **Thread resolution policy**: Resolve only for `ai-bot`; keep open for `human`.

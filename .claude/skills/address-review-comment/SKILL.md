---
name: address-review-comment
description: "Address a single GitHub PR review comment end-to-end. Use when the user provides a link to a PR review comment. Reads the comment via gh CLI (never fetch_webpage), evaluates validity, applies fixes, commits, drafts a reply, and posts it."
disable-model-invocation: true
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
- `id` — the REST numeric ID of the comment
- `user.login` / `user.type` — reviewer identity and actor type
- `path` — the file the comment refers to
- `line` / `original_line` — the line number
- `diff_hunk` — the surrounding diff context
- `in_reply_to_id` — if this is part of a thread, the parent comment ID

After extraction, set a tentative reply target type without interrupting the user:

- Prefer `ai-bot` when any of the following is true:
  - `user.type == "Bot"`
  - `user.login` matches common bot naming patterns (e.g., ends with `[bot]`)
- Otherwise, set `human`.

Treat this as provisional and finalize at Step 7.

If the comment is part of a thread, also fetch the full thread:

```bash
gh api repos/{owner}/{repo}/pulls/{number}/comments \
  | jq '[.[] | select(.in_reply_to_id == {parent_id} or .id == {parent_id})]'
```

### 3. Understand the Context

- Read the file and lines the comment refers to.
- Understand the reviewer's suggestion or concern.
- Look at surrounding code for broader context if needed.

### 4. Evaluate Validity

Use the `address-finding` skill (invoke it via the Skill tool) to evaluate the comment.
It carries the full judgment criteria, the verdict options (valid / valid-but-not-fundamental / partially valid / not valid), and the anti-patterns.
Land on a verdict before proceeding.

### 5. Apply Fix (if needed)

If a fix is warranted, apply it under the `address-finding` skill: root-cause and minimal scope, its consistency sweep across related sites, and its reflective band-aid checkpoint.
Complete all file edits in this step before proceeding.

### 6. Draft Reply

Compose a reply draft early, before final confirmation:

- Write in the **same language as the reviewer's comment**.
- Select tone by reply target type:
  - `human`: concise, professional, and courteous (brief appreciation is allowed).
  - `ai-bot`: concise, professional, and factual (avoid unnecessary pleasantries).
- If a fix is planned or made: acknowledge and briefly describe the change, then include a placeholder on a new line (e.g., `\n\nFixed in <commit_sha>.`).
- If no fix will be made: explain the reasoning respectfully.

### 7. Confirm Fix with User

Present the diff and reasoning to the user as a **single final execution checkpoint**.

Checklist:

- Precondition: if code changes are required, confirm the working tree already contains the intended edits; otherwise, return to Step 5 before asking for approval.
- Show what was changed and why.
- If no change was made, explain why the suggestion was declined or deferred.
- Show inferred reply target type (`human` or `ai-bot`) and finalize it here together with commit/push confirmation.
- Ask for override only when the user disagrees or the inference confidence is low.
- Show the Step 6 draft reply that will be posted after commit/push.

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
- Keep the description under 72 characters.
- Stage only the files related to this review comment.
- Push to the current branch so the commit is visible on the PR.

### 9. Post Reply

Post the approved reply with `gh` CLI, replying to the review comment thread.
Write the approved reply body to a temporary file first, then pass it with `-F body=@<file>` so backticks, `$`, and newlines in the reply are never interpreted by the shell:

```bash
gh api repos/{owner}/{repo}/pulls/{number}/comments/{comment_id}/replies \
  -F body=@<reply-body-file>
```

- Replace `<commit_sha>` in the body with the actual SHA from Step 8 when a fix was made, before writing the file.
- Do not inline the body with `-f body="..."`: reply text often quotes code, which the shell would expand or mangle.

After posting:

- If reply target type is `human`: do not resolve the thread.
- If reply target type is `ai-bot`: resolve the thread.

Resolve flow for `ai-bot` — resolve the review thread via GraphQL:

- Fetch the PR's review threads and map `comment_id` to its thread:

  ```bash
  gh api graphql -f owner={owner} -f repo={repo} -F number={number} -f query='
    query($owner:String!, $repo:String!, $number:Int!) {
      repository(owner:$owner, name:$repo) {
        pullRequest(number:$number) {
          reviewThreads(first:100) {
            nodes { id isResolved viewerCanResolve comments(first:100) { nodes { databaseId } } }
          }
        }
      }
    }'
  ```

- Select the thread whose `comments.nodes[].databaseId` includes `comment_id`, and read its `id` (the GraphQL thread ID).
- If the thread is found, not already `isResolved`, and `viewerCanResolve` is `true`, resolve it:

  ```bash
  gh api graphql -f threadId={thread_id} -f query='
    mutation($threadId:ID!) {
      resolveReviewThread(input:{threadId:$threadId}) { thread { id isResolved } }
    }'
  ```

- If no matching thread is found, or `viewerCanResolve` is `false`, skip resolving and keep only the posted reply.

## Important Rules

1. **GitHub Private Repo Policy**: NEVER use `fetch_webpage` or browser tools for GitHub URLs. Always use the `gh` CLI.
2. **One comment at a time**: Handle a single comment per invocation.
3. **Single final checkpoint**: Ask once at Step 7, then run commit/push and posting in sequence.
4. **Commit language**: Always English, conventional commit format.
5. **Reply language**: Always match the reviewer's comment language.
6. **Commit hash in reply**: Always include the commit SHA when a fix was made.
7. **Reply target type**: Infer after reading comment metadata, then finalize at Step 7 (single checkpoint).
8. **Thread resolution policy**: Resolve only for `ai-bot`; keep open for `human`.

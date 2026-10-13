---
name: address-review-comment
description: "Address GitHub PR review comments end-to-end: one comment, one review, or every open thread on a PR. Use when the user provides a link to a PR review comment, a review, or a PR. Reads each comment via gh CLI (never WebFetch or fetch_webpage), evaluates validity, applies fixes, commits, drafts a reply, and posts it."
disable-model-invocation: true
---

# Address Review Comment

Handle a GitHub PR review comment end-to-end: read → evaluate → fix → commit → reply.

## Input

One of three GitHub URLs:

- A review comment, addressed on its own:
  - `https://github.com/{owner}/{repo}/pull/{number}#discussion_r{comment_id}`
  - `https://github.com/{owner}/{repo}/pull/{number}/files#r{comment_id}`
- A review, `https://github.com/{owner}/{repo}/pull/{number}#pullrequestreview-{review_id}`: the open threads that review started, and what its summary says beyond them.
- A PR, `https://github.com/{owner}/{repo}/pull/{number}`: every open thread on it.

The Procedure is written for one comment; [Several comments at once](#several-comments-at-once) says what a review or a PR changes in it.

## Procedure

**Every GitHub read and write below goes through the `gh` CLI, not through a web-fetch tool (`WebFetch`, `fetch_webpage`) or a browser tool.**

### 1. Parse the URL

Extract from the comment link:

- `owner`, `repo`, PR `number`
- `comment_id` from the URL fragment (`#discussion_r{id}` or `#r{id}`)

### 2. Read the Comment

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

Fetch the thread the comment belongs to, whether it is the thread's first comment or a reply; `{root_id}` is its `in_reply_to_id`, or its own `id` where that is null:

```bash
gh api --paginate repos/{owner}/{repo}/pulls/{number}/comments \
  --jq '.[] | select(.id == {root_id} or .in_reply_to_id == {root_id})'
```

Then set a tentative reply target type without interrupting the user, from who wrote the thread's comments:

- `ai-bot` where a bot started the thread and every later comment is a bot's or this account's (`gh api user --jq .login`). A comment is a bot's where either holds:
  - `user.type == "Bot"`
  - `user.login` matches common bot naming patterns (e.g., ends with `[bot]`)
- `human` otherwise, so a thread a bot started and someone else then spoke in stays open.

Treat this as provisional and finalize at Step 7.

### 3. Understand the Context

- Read the file and lines the comment refers to.
- Understand the reviewer's suggestion or concern.
- Look at surrounding code for broader context if needed.
- Read the PR's own title and body for what the change is for:

  ```bash
  gh api repos/{owner}/{repo}/pulls/{number} --jq '{title, body}'
  ```

  - This is what this workflow offers for `address-finding`'s purpose source, used unless the user states one.
  - The comment sets what to fix; the PR sets how far a fix may reach.
- When the title and body state no intent, read the change as a whole instead.
  - `gh pr diff {number} --repo {owner}/{repo} --name-only` is the cheap first look; read the patch itself when the file list says too little to bound anything.
  - Work an earlier invocation landed past the bound may be fixed, but it does not widen a purpose inferred here — otherwise each approved excursion raises the bound for the next comment.
- State the purpose and which source it came from; that satisfies `address-finding`'s purpose statement, so it is not repeated at Step 5.

### 4. Evaluate Validity

Use the `address-finding` skill (invoke it via the Skill tool) to evaluate the comment.
It carries the judgment criteria and the anti-patterns.
Land on a verdict before proceeding.

### 5. Apply Fix (if needed)

If a fix is warranted, apply it under the whole of the `address-finding` skill's "The purpose bound", "Putting the excess to the user", "Choose the fix" and "Apply the fix" sections, fed by Step 3.
For that skill's no-silent-reversal check, the piece of work is this PR branch: check the fix against the commits already on it.
Complete all file edits in this step before proceeding.

The part of a fix that reaches past the purpose bound is the one thing this skill asks about before Step 7.

- Present the options and your recommendation, and wait.
- The part of the fix inside the bound lands here whichever option is chosen, and so does any correction the core lands outside it on its own; only the part past the bound depends on the answer.
  - Extend the change: that part lands here too.
  - Carve it out: it is left, to be done separately.
  - Leave it: it is left undone.
- Report the option taken at Step 7.

### 6. Draft Reply

Compose a reply draft early, before final confirmation:

- Write in the **same language as the reviewer's comment**.
- Select tone by reply target type:
  - `human`: concise, professional, and courteous (brief appreciation is allowed).
  - `ai-bot`: concise, professional, and factual (avoid unnecessary pleasantries).
- If the whole fix landed as the comment proposed: acknowledge and briefly describe the change, then include a placeholder on a new line (e.g., `\n\nFixed in <commit_sha>.`).
- If any part did not land, anything was surfaced rather than fixed, or a fix of another shape answered the comment: say plainly what was left out or taken instead and why, and whether anything is to be done separately.
  - Describe what did land and keep the placeholder; when nothing landed at all, drop the placeholder.
- If the comment was declined on the merits: explain the reasoning respectfully.

### 7. Confirm Fix with User

Present the diff and reasoning to the user as a **single final execution checkpoint**.

Checklist:

- Precondition: if a fix was warranted, confirm the working tree holds every edit that was meant to land; otherwise, return to Step 5 before asking for approval. Those are:
  - the part inside the purpose bound, always;
  - a correction `address-finding` landed outside the bound on its own;
  - the part past the bound, only where the user chose to extend the change.
- Show what was changed and why.
- Call out anything the fix reached beyond what the comment asked for, or took a different shape from what it proposed, so the user does not have to spot it in the diff.
- Explain anything declined or deferred, whether or not other edits landed — a part the purpose bound held back, and anything surfaced rather than fixed.
- Show inferred reply target type (`human` or `ai-bot`) and whether the thread will be resolved, and finalize both here together with commit/push confirmation. Work left for later is a reason to keep the thread open, so say what was left.
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
git diff --quiet @{push} -- <changed-files> && git rev-parse --short @{push}
```

- Keep the description under 72 characters.
- Stage only the files related to this review comment.
- Push to the current branch so the commit is visible on the PR.
- The last command prints the SHA for Step 9, and only once the pushed branch holds the edits.
  - Where it prints nothing, the commit or the push did not land: settle that before Step 9 rather than citing an earlier commit.

### 9. Post Reply

Post the approved reply with `gh` CLI, replying to the review comment thread.
Write the approved reply body to a temporary file first, then pass it with `-F body=@<file>` so backticks, `$`, and newlines in the reply are never interpreted by the shell:

```bash
gh api repos/{owner}/{repo}/pulls/{number}/comments/{comment_id}/replies \
  -F body=@<reply-body-file>
```

- Replace `<commit_sha>` in the body with the actual SHA from Step 8 when a fix was made, before writing the file.

After posting:

- If reply target type is `human`: do not resolve the thread, which Step 7 cannot override.
- If reply target type is `ai-bot`: resolve the thread, unless Step 7 settled otherwise.

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

## Several comments at once

A review URL or a PR URL names a set of threads.
Each open one goes through the Procedure as its own comment, with the differences this section states.

### Which threads are open

List the PR's threads once:

```bash
gh api graphql -f owner={owner} -f repo={repo} -F number={number} -f query='
  query($owner:String!, $repo:String!, $number:Int!) {
    viewer { login }
    repository(owner:$owner, name:$repo) {
      pullRequest(number:$number) {
        reviewThreads(first:100) {
          nodes {
            id isResolved viewerCanResolve
            comments(first:100) { nodes { databaseId body author { __typename login } pullRequestReview { databaseId } } }
          }
        }
      }
    }
  }'
```

- A thread is open while it is not `isResolved` and its last comment's author is not `viewer.login`.
  - One this account answered last is waiting on the reviewer, so it is left out.
- A review URL keeps only the threads whose first comment has `pullRequestReview.databaseId` equal to `review_id`; a reply carries a review ID of its own.
- A thread's first comment is the one Step 2 reads and Step 9 replies to.
  - The listing already holds what Step 2's thread fetch returns, so take the thread's comments from it, and a bot's from `author.__typename` being `Bot`.
  - What is answered is what the reviewer said since this account's last reply, or the whole thread where there is none.
- Where no thread is open, say so and stop, unless the URL is a review's and its summary carries a point of its own.

### A review's summary

Read the review with `gh api repos/{owner}/{repo}/pulls/{number}/reviews/{review_id}`.

- A point its `body` makes by linking a thread is that thread, and takes the thread's verdict rather than a second one, whether the thread is open or already answered.
- A point no thread carries goes through Steps 3-5 as a comment would.
  - It has no thread, so Step 6 drafts nothing for it and Step 9 posts nothing; Step 7 reports its verdict.

### What changes in the steps

- Step 3 reads the PR's purpose once for the set.
- Step 7 stays the single checkpoint, showing every comment's verdict, diff and draft reply together.
- Step 8 commits once per comment whose fix landed, and pushes once.
  - Two fixes that cannot be staged apart share a commit, which both replies cite.
  - A commit that fails stops the sequence before the next `git add`: its files stay staged, and the next commit would carry them under its own subject.
  - Before the push, `git log --reverse --format='%h %s' @{push}..HEAD` lists the commits it will carry: each reply cites the one holding its fix, and a fix with no commit there did not land.
  - Step 8's check runs over every changed file and gates Step 9 as before.
- Step 9 posts each reply to its own thread and applies the resolution policy thread by thread, resolving by the `id` and `viewerCanResolve` the listing returned rather than fetching the threads again.

### Asking Copilot for another pass

Where a thread answered here is Copilot's, Step 7 also asks whether to request its re-review.
The request follows Step 9:

```bash
gh pr edit {number} --repo {owner}/{repo} --add-reviewer @copilot
```

- The command can exit 0 with nothing registered, so list the `review_requested` events before and after it and confirm a new one:

  ```bash
  gh api --paginate repos/{owner}/{repo}/issues/{number}/timeline \
    --jq '.[] | select(.event == "review_requested") | .created_at'
  ```

- Where none appears, ask the user to request it from the PR page rather than retrying.

## Important Rules

These two are repeated from the steps above, and nothing else belongs here: breaking one commits an act outside this machine that no later step can take back.
Routing every GitHub read and write through the `gh` CLI is the third of that kind and is not repeated: the Procedure lead carries it ahead of every step, which is the most prominent place this file has.

1. **Single final checkpoint**: Ask once at Step 7, then run commit/push and posting in sequence.
   - The sole earlier question concerns the part of a fix that reached past the purpose bound at Step 5, whose answer decides what Step 7 will show.
2. **Thread resolution policy**: Resolve only for `ai-bot`, unless Step 7 settled otherwise; keep open for `human`, which Step 7 cannot override.

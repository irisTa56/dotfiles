---
name: post-merge-continue
description: Ready the session to keep working after a pull request merges — write down what the continuation needs, then bring the base branch up to date locally. Invoke when the user reports that a PR merged and the work goes on here.
argument-hint: "[head branch of the merged PR] — omit to use the current checkout"
disable-model-invocation: true
---

# Post-merge continue

The merge is where a session gets compacted, so this readies it for that: what the continuation needs is written down while the context still holds it, and the base branch is brought up to date locally.

Invoking this is the request and the permission; do not ask again before each step.

## 1. Write down what the continuation needs

A compaction summarizes, so anything that has to survive it intact reaches a file first.
Write to the scratchpad directory this session was given for temporary files, or to the operating system's temporary directory where it was given none.
Name it `post-merge-continue.md`, so the continuation can find it by convention when the path does not survive the summary.
That file outlives the compaction and not the session, which is the lifetime this needs.

Write down what the next stretch of work needs, not what would be lost.
Compacting is a deliberate act signalling intent to keep going here, so that work is the bound; loss has none.

- Where the work stood, and what it was about to do next.
- An alternative that was tried and rejected, with the reason, so the continuation does not propose it again.
- A deliberate omission, so the continuation does not helpfully restore it.

Name its path in your reply before going on to step 2, since every step below can stop early and the compaction will not wait for a report that never comes.
What has to outlive the session is not this skill's to write; `handoff` produces that document.

## 2. Read the base branch from the merged pull request

The head branch is the argument where one was given, and otherwise the current checkout: `git rev-parse --abbrev-ref HEAD`.
Ask for it wherever neither yields a branch name, as on a detached checkout, which returns the literal `HEAD`.

`gh pr list --head <head> --state merged --limit 1 --json baseRefName`

An empty result means no merged pull request with that head branch here, so stop and say which branch you queried.
A squash merge rewrites the commits, so the branch stays unreachable from its base and [`git branch --merged` cannot see it](https://git-scm.com/docs/git-branch).
The base is not always the default branch, since a stacked pull request merges into another feature branch.

## 3. Fast-forward the base branch

`git fetch origin <base>:<base>` advances the local ref without switching, and is the whole step wherever no worktree has the base checked out.
Name the remote rather than leaving it out, since [`git fetch` without one follows the current branch's upstream](https://git-scm.com/docs/git-fetch) and the base need not share it.

Where a worktree does hold the base, that same command refuses and names the worktree in the refusal:

```text
fatal: refusing to fetch into branch 'refs/heads/main' checked out at '<path>'
```

Take the path from there rather than from `git worktree list`, which inside a submodule names the gitdir instead of the working tree.
Then fast-forward that checkout, fetching first because the refused command left the remote-tracking ref untouched:

- `git fetch origin <base>`
- `git -C <path> merge --ff-only origin/<base>`

Nothing here forces a base carrying local-only commits: the colon form refuses a non-fast-forward for want of a `+` on its refspec, and [`--ff-only` refuses one by definition](https://git-scm.com/docs/git-merge).
The merge can also stop over that worktree's own state, and git says why whenever it stops, so report the reason rather than working around it.
Uncommitted work there does not on its own hold the branch back, so do not read the merge as guarded.

## 4. Report

Say what the base branch moved to, or that it did not and why.
Say which branch this checkout is on, and whether it is the merged head, since the continuation will want a fresh branch off the base.

The merged branch and its worktree are left alone, for `gh-poi` to sweep with the rest.

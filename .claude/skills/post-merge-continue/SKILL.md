---
name: post-merge-continue
description: Ready the session to keep working after a pull request merges — write down what the continuation needs, then bring the base branch up to date locally. Invoke when the user reports that a PR merged and the work goes on here.
argument-hint: "[head branch of the merged PR, or what the next stretch of work is for] — omit for neither"
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

- Where the work stood, and what it was about to do next, which an argument naming no branch is the user saying.
- An alternative that was tried and rejected, with the reason, so the continuation does not propose it again.
- A deliberate omission, so the continuation does not helpfully restore it.

Name its path in your reply before going on to step 2, since every step below can stop early and the compaction will not wait for a report that never comes.
What has to outlive the session is not this skill's to write; `handoff` produces that document.

## 2. Read the base branch from the merged pull request

The head branch is the argument where that names one, and otherwise the current checkout: `git rev-parse --abbrev-ref HEAD`.
An argument naming no branch belongs to step 1 and never reaches the query below, which would take it and return empty, reading as no merged pull request.
Ask for the branch wherever neither source yields one, as on a detached checkout, which returns the literal `HEAD`.

`gh pr list --head <head> --state merged --limit 1 --json baseRefName`

An empty result means no merged pull request with that head branch here, so stop and say which branch you queried.
The base is not always the default branch, since a stacked pull request merges into another feature branch.

## 3. Fast-forward the base branch

`git fetch origin <base>:<base>` advances the local ref without switching, and is the whole step wherever no worktree has the base checked out.
Name the remote rather than leaving it out, since [`git fetch` without one follows the current branch's upstream](https://git-scm.com/docs/git-fetch) and the base need not share it.
Where a command stops for a reason this step does not answer below, report what git says rather than working around it.

Where a worktree does hold the base, that same command refuses and the refusal carries a path:

```text
fatal: refusing to fetch into branch 'refs/heads/main' checked out at '<path>'
```

Take that path as the refusal gives it, rather than resolving it to a checkout first.
Where a submodule's main worktree holds the base it is the gitdir and not the working tree, and `git -C` still reaches the working tree from there through [`core.worktree`](https://git-scm.com/docs/git-config#Documentation/git-config.txt-coreworktree).
Then fast-forward that checkout, fetching first because the refused command left the remote-tracking ref untouched:

- `git fetch origin <base>`
- `git -C <path> merge --ff-only origin/<base>`

## 4. Report

Say what the base branch moved to, or that it did not and why.
Say which branch this checkout is on, and whether it is the merged head, since the continuation will want a fresh branch off the base.

The merged branch and its worktree are left alone, for `gh-poi` to sweep with the rest.

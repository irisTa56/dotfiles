---
name: sync-base
description: Bring a base branch up to date with origin locally, including when this session runs in a worktree and the base is checked out in another one. Use when the user reports that a pull request merged and work continues, or asks to update or sync the base branch.
argument-hint: "[base branch] — omit for the merged pull request's base, or origin's default branch"
allowed-tools: Bash(${CLAUDE_SKILL_DIR}/scripts/sync-base.sh *)
---

# Sync base

The base is the argument where one is given.
Otherwise, where a pull request from the current branch has merged, take its base, since a stacked pull request merges into another feature branch rather than the default one:

`gh pr list --head <current branch> --state merged --limit 1 --json baseRefName`

Take the current branch from `git branch --show-current`, and skip the query where that prints nothing, as on a detached checkout: an empty `--head` filters nothing and returns whichever pull request merged last.
Where neither yields a base, leave it out, and the script takes origin's default branch.

Run `${CLAUDE_SKILL_DIR}/scripts/sync-base.sh <base>`.
It switches this checkout to the base and fast-forwards it, or, where another worktree has the base checked out, fast-forwards that worktree and leaves this checkout where it is.
Where it fails, report what git says rather than working around it.

Report what the base moved to, and which branch this checkout is on, since continuing work wants a fresh branch off the base.
The merged branch and its worktree are left alone, for `gh-poi` to sweep with the rest.

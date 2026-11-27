---
name: sync-base
description: Bring a base branch up to date with origin locally, including when this session runs in a worktree and the base is checked out in another one. Use when the user reports that a pull request merged and work continues, or asks to update or sync the base branch.
argument-hint: "[base branch] — omit for the merged pull request's base, or origin's default branch"
allowed-tools: Bash(~/.local/bin/dotfiles-local git-sync-base *)
disable-model-invocation: true
---

# Sync base

The base is the argument where one is given.
Otherwise, where a pull request from the current branch has merged, take its base, since a stacked pull request merges into another feature branch rather than the default one:

```shell
gh pr list --head <current branch> --state merged --limit 1 --json baseRefName
```

Take the current branch from `git branch --show-current`, and skip the query where that prints nothing, as on a detached checkout: an empty `--head` filters nothing and returns whichever pull request merged last.
Where neither yields a base, leave it out, and the command takes origin's default branch.

Run `~/.local/bin/dotfiles-local git-sync-base <base>`.
It switches this checkout to the base and brings it up to origin, or, where another worktree has the base checked out, brings that worktree's up and leaves this checkout where it is.
It fast-forwards, or, where the local base has commits origin lacks and origin has moved on as well, rebases those commits onto origin's base.
Where it fails, report what git says rather than working around it.

Report what the base moved to, and which branch this checkout is on, since continuing work wants a fresh branch off the base.
Where it rebased commits that only the local base had, report their hashes before and after as it prints them, since whoever made those commits may have cited the old ones.
The merged branch and its worktree are left alone, for `sweep-branches` to clear with the rest.

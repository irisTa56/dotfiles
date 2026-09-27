---
name: sweep-branches
description: Delete the local branches whose work has reached the default branch, including the ones gh-poi keeps because no pull request holds their commits, then look into the branches and linked worktrees left and propose a keep-or-delete call on each, with its evidence.
allowed-tools: Bash(mise run git:sweep)
disable-model-invocation: true
---

# Sweep branches

Run `mise run git:sweep` and report what it deleted, with the SHA it prints for each branch, since nothing else brings one back.
It runs `gh-poi`, which deletes a branch its merged pull request holds, restorable from that pull request, and force-removes that branch's worktree, ignored files included, unless it has changes or untracked files.
It then deletes every branch the default branch already holds whole, except one checked out in a worktree or held by `gh-poi lock`.
Where the task fails, report what it says rather than working around it.

## What is left

Look into each local branch and linked worktree left, and close each with delete or keep, the evidence, and, where the evidence leaves it open, what would settle it.

Leave these out:

- a branch `gh-poi lock` holds;
- a worktree in use, and the branch checked out in it.
  - A worktree is in use where this session runs in it, or where a session that is not archived has its `cwd` in it.
  - List those sessions with the desktop app's `list_sessions` tool (`mcp__ccd_session_mgmt__list_sessions`), raising its `limit` until fewer come back than it allows, since it lists 20 by default and scheduled runs alone can number hundreds; where the tool is not available, ask the user.

For a branch, look at its commits the default branch does not hold:

- A pull request squash-merged into the default branch holds every commit up to its head, though no single one of several matches the squash commit patch for patch.
- A pull request merged into another branch, as a stacked one is, says nothing about the default branch.
- A commit may have landed under another one, as when the work was redone in a later pull request.

For a worktree, look at what removing it discards:

- changed and untracked paths;
- ignored paths, which `git worktree remove` deletes without asking: output a command regenerates, such as caches, build output or installed dependencies, or data nothing regenerates, such as local settings, credentials or a tool's session records;
- on a detached HEAD, commits no branch holds.

A branch checked out in a worktree can be deleted only once that worktree is removed, but its call still comes from its own commits.

## Acting on the answer

Act only on what the user names, which may come later in the session.
Remove a worktree with `git worktree remove <path>`, adding `--force` only where they named it knowing what it holds.
Delete a branch with `git branch -D <branch>`, and report each one's last SHA.

---
name: sweep-branches
description: Delete the local branches whose work has reached the default branch, including the ones gh-poi keeps because no pull request holds their commits, and propose a keep-or-delete call on each branch and linked worktree left, with its evidence.
allowed-tools: Bash(mise run git:sweep)
disable-model-invocation: true
---

# Sweep branches

Run `mise run git:sweep`.
It runs `gh-poi`, which also removes a clean worktree on a merged branch, then deletes every branch the default branch already holds whole.
It lists the rest under `Left for a decision`, each with its commits missing from the default branch and its pull requests, and then the linked worktrees left.
Report what the two deleted, with the SHA the output gives for each branch the task deleted, since nothing else brings one back; a branch `gh-poi` deleted can be restored from its pull request.
Where the task fails, report what it says rather than working around it.

## Worktrees

Claude Code reuses a worktree across sessions, and one whose last session was archived stays behind, usually on a detached HEAD, which `gh-poi` never touches.
List the sessions that are not archived with the desktop app's `list_sessions` tool (`mcp__ccd_session_mgmt__list_sessions`), raising its `limit` until fewer come back than it allows, since it lists 20 by default and scheduled runs alone can number hundreds.
Take a worktree as in use where one of them has its `cwd` in it, or where this session runs in it.
Where that tool is not available, ask the user which worktrees are in use.

Look into each worktree not in use along these, and report what each shows:

- Changed and untracked paths, from `git -C <path> status --short`: what they are.
- Ignored paths, from `git -C <path> status --short --ignored`, which `git worktree remove` deletes without asking: which are output a command regenerates, such as caches, build output or installed dependencies, and which are data nothing regenerates, such as local settings, credentials or a tool's session records.
- On a detached HEAD, the commits no branch holds, from `git -C <path> log --oneline HEAD --not --branches --remotes`, which removing the worktree discards.
- On a branch, that branch's call below.

Close each with remove or keep and the reason, and where the viewpoints leave it open, say what would settle it.
A worktree in use is noted without looking into it.

## Branches

Then give each listed branch one line: delete or keep, and the evidence the call rests on.

- A `locked` branch is kept on purpose; leave it out of the proposals.
- A branch checked out in a worktree in use is left out of the proposals.
- A branch checked out in any other worktree can be deleted only once that worktree is removed, but its call still comes from the rules below.
- A branch with no missing commits has all its work on the default branch, through its merged pull request or as equivalent patches; propose deleting it.
  - Where a pull request under its name merged into the default branch, the task counts only the commits after that pull request's head, since a squash merge matches none of several commits patch for patch.
- For a branch with missing commits, read each one with `git show --stat` and look for its change on the default branch under another commit, as when the work was redone in a later pull request.
  - Where you find it, cite that commit.
  - Where you do not, propose keeping the branch and say what the commits hold, since deleting it discards them.

## Acting on the answer

Act only on what the user then names, which may come later in the session.
Remove a worktree the user named with `git worktree remove <path>`, adding `--force` only where they named it knowing it has changes.
Delete a branch with `git branch -D <branch>`, and report each one's last SHA; a branch still checked out in a worktree the user did not name stays.

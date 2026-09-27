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
List the sessions with the desktop app's `list_sessions` tool (`mcp__ccd_session_mgmt__list_sessions`), archived ones included, and take a worktree as in use where a session that is not archived has its `cwd` in it, or where this session runs in it.
Where that tool is not available, ask the user which worktrees are in use.

Give each worktree not in use one line: remove or keep, and the evidence.

- Propose removing one with no changed and no untracked paths, and, where it is detached, no commits on no branch.
- For one with changed or untracked paths, read `git -C <path> status` and say what is there; propose removing it only where what is there is output a command regenerates.
- For a detached one with commits on no branch, list them with `git -C <path> log --oneline HEAD --not --branches --remotes` and propose keeping it, since removing it discards them.

A worktree in use is noted without a proposal.

## Branches

Then give each listed branch one line: delete or keep, and the evidence the call rests on.

- A `locked` branch is kept on purpose; leave it out of the proposals.
- A branch checked out in a worktree can be deleted only once that worktree is removed, but its call still comes from the rules below.
- A branch with no missing commits has all its work on the default branch, through its merged pull request or as equivalent patches; propose deleting it.
  - Where a pull request under its name merged into the default branch, the task counts only the commits after that pull request's head, since a squash merge matches none of several commits patch for patch.
- For a branch with missing commits, read each one with `git show --stat` and look for its change on the default branch under another commit, as when the work was redone in a later pull request.
  - Where you find it, cite that commit.
  - Where you do not, propose keeping the branch and say what the commits hold, since deleting it discards them.

## Acting on the answer

Act only on what the user then names.
Remove a worktree with `git worktree remove <path>`, adding `--force` only for one the user named knowing it has changes.
Delete a branch with `git branch -D <branch>`, after removing any worktree it is checked out in, and report each one's last SHA.

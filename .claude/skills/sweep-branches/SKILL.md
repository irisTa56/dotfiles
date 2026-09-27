---
name: sweep-branches
description: Delete the local branches whose work has reached the default branch, including the ones gh-poi keeps because no pull request holds their commits, and propose a keep-or-delete call on each branch left, with its evidence.
allowed-tools: Bash(mise run git:sweep)
disable-model-invocation: true
---

# Sweep branches

Run `mise run git:sweep`.
It runs `gh-poi`, then deletes every branch the default branch already holds whole, and lists the rest under `Left for a decision`, each with its commits missing from the default branch and its pull requests.
Report what the two deleted, with the SHA the output gives for each branch the task deleted, since nothing else brings one back; a branch `gh-poi` deleted can be restored from its pull request.
Where the task fails, report what it says rather than working around it.

Then give each listed branch one line: delete or keep, and the evidence the call rests on.

- A `locked` branch is kept on purpose; leave it out of the proposals.
- A branch checked out in a worktree stays, since removing the worktree may break the session that uses it; note it without a proposal.
- A branch with no missing commits has all its work on the default branch, through its merged pull request or as equivalent patches; propose deleting it.
  - Where a pull request under its name merged, the task counts only the commits after that pull request's head, since a squash merge matches none of several commits patch for patch.
- For a branch with missing commits, read each one with `git show --stat` and look for its change on the default branch under another commit, as when the work was redone in a later pull request.
  - Where you find it, cite that commit.
  - Where you do not, propose keeping the branch and say what the commits hold, since deleting it discards them.

Delete only the branches the user then names, with `git branch -D <branch>`, and report each one's last SHA.

---
name: post-merge-cleanup
description: Close out a merged pull request — record what the repository will not remember, fast-forward the base branch, and delete the merged branch locally and on the remote. Invoke when the user reports that a PR merged.
disable-model-invocation: true
---

# Post-merge cleanup

Invoking this is the request and the permission; do not ask again before each step.
That permission covers step 4's deletion on the remote, which `~/.claude/INSTRUCTIONS.md` would otherwise hold for confirmation as a write to an external system.

1. Record what the next stretch of work needs and the repository will not hold.
   - Write it into the file that work will read, not the merged PR body, which it will not open.
   - Alternatives that were tried and rejected go there with the reason, so a later pass does not propose them again.
   - Deliberate omissions go there too, so a later pass does not helpfully restore them.
   - The merge is a boundary, and the session holding the reasoning may not outlive it.
   - Write for the continuation rather than for an archive, since only what reached a file is yours to control.
2. Confirm the merge and read the base branch.
   - `gh pr list --head <branch> --state merged --limit 1 --json number,baseRefName,headRefOid`
   - An empty result means no merged PR, so stop and say what you found.
   - A squash merge rewrites the commits, so the branch stays unreachable from its base.
   - `git branch --merged` and `git cherry` therefore cannot see the merge and are not the test.
   - The base is not always the default branch, because a stacked PR merges into another feature branch.
3. Fast-forward the base branch.
   - `git fetch --prune` first, since the merge form below reads a remote-tracking ref that step 2 never refreshed.
   - `git -C <the worktree holding the base> merge --ff-only origin/<base>` when any worktree has it checked out, which need not be this one.
   - `git fetch origin <base>:<base>` when none does, which advances the local ref without switching.
   - Fetching into a branch fails wherever a worktree holds it, so the first form is not only for the base checked out here.
   - Both forms refuse a non-fast-forward, so a base carrying local-only commits is reported rather than rewritten.
4. Delete the branch.
   - `git branch -D <branch>` locally, since the squash merge above makes `-d` refuse.
   - `-D` is safe only because step 2 proved the merge, and only while the branch still points at `headRefOid`.
   - A tip past that commit is work added after the merge, which no proof covers; keep the branch and say so.
   - A worktree holding the branch blocks the deletion, and removing it is not this command's work.
   - Leave both to `cleanup-merged-branches`, unless the holder is the main working tree, which nothing removes; switch that checkout off the branch instead.
   - Delete-on-merge has often removed the branch already, and the fetch in step 3 prints only changes, so it is no test of what is still there.
   - `git push origin --delete <branch>` only when `git show-ref --verify --quiet refs/remotes/origin/<branch>` succeeds.
5. Report what was removed, and what was skipped and why.

# The private workspace

Working files, such as research notes and code written only to try something out, are kept outside the repository's working tree, in a checkout of a private repository that accompanies the repository: its companion repository.
Where the repository uses Entire, its checkpoints go to that same repository ([entire.md](entire.md#visibility)).

`git config --get --type=path dev-docs.private-workspace` prints the checkout's directory, from any worktree of the clone.
Where it prints nothing, ask the person for the companion repository's URL and for a directory outside every git working tree, clone it there, and record the directory under that key, rather than keeping working files untracked in the repository.

One checkout serves every session:

- Keep a branch's files under a subdirectory named `<YYYYMMDD>-<branch>`, dated the day it is created, which stays after the branch merges.
- Commit there as the work goes, naming the paths you wrote: `git add <path>`, then `git commit -- <path>`, which [leaves whatever another session has staged out of your commit](https://git-scm.com/docs/git-commit#Documentation/git-commit.txt---only).
  - When the repository's branch is pushed, offer to push the workspace as well, since until then this checkout is the only copy of what it holds.

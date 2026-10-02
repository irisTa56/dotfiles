# The private workspace

Working files, meaning research notes and code written only to try something out, are kept outside the repository's working tree, in a checkout of a private repository that accompanies the repository: its companion repository.
Where the repository uses Entire, the companion repository is also where its checkpoints go ([entire.md](entire.md#visibility)), so a session's transcript and the files it worked on sit in one private place.
No file committed to the repository names the companion repository; each clone of the repository holds that in its git config, which linked worktrees share.

## Finding it

`git config --get --type=path dev-docs.private-workspace` prints the workspace's directory.
If it prints nothing, set the clone up rather than keeping working files untracked in the repository:

1. Ask the person for the companion repository's URL, and for the directory to clone it to, which must lie outside every working tree of the repository.
   - Where they already have a checkout, take its directory instead.
2. Clone it there, and record the directory with `git config dev-docs.private-workspace <path>`.
3. Where the repository uses Entire, give the clone its `checkpoints` remote from the same URL, as [entire.md](entire.md#visibility) lists.

## Layout

- `<branch>/`: one subdirectory for each branch of the repository, named after it, holding that branch's working files.
- `carry-over/`: findings kept for a later phase, shared by every branch.

## Working in it

One checkout serves every session, so other sessions work beside yours:

- Write only under your branch's subdirectory and `carry-over/`.
- Commit there as the work goes, naming the paths you wrote: `git add <path>`, then `git commit -- <path>`, which leaves whatever another session has staged out of your commit.
  - A commit that fails because another git process holds the index lock has changed nothing, so run it again.
- List environments and build output, such as `.venv/` and `target/`, in the workspace's `.gitignore`, so that `git add` leaves them out.
- Delete ignored files only when the person asks for the disk space back, and then with `git clean -fdX -- <subdirectory>`, which removes the ignored files under that one subdirectory and nothing else.
  - It removes every ignored file there, a session's environment included, and no commit holds them to restore from.

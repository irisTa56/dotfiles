# The private workspace

Working files, meaning research notes and code written only to try something out, are kept outside the repository's working tree, in a checkout of a private repository that accompanies the repository: its companion repository.
Where the repository uses Entire, its checkpoints go to that same repository ([entire.md](entire.md#visibility)).

## Setting a clone up

No file committed to the repository names the companion repository, so each clone is set up once, with [scripts/companion.exs](../scripts/companion.exs):

- `elixir <path to it> check` exits 0 where the clone is set up, and otherwise lists what is missing.
- Where something is missing, ask the person for the companion repository's URL and for a directory outside the repository to keep its checkout in, and run `elixir <path to it> setup <url> <directory>`, rather than keeping working files untracked in the repository.
  - It clones the companion repository there unless the directory already exists, and records everything in the clone's git config, which linked worktrees share.

`git config --get --type=path dev-docs.private-workspace` then prints the workspace's directory.

## Layout

- `<branch>/`: one subdirectory for each branch of the repository, named after it, holding that branch's working files.
- `carry-over/`: findings kept for a later phase, shared by every branch.

## Working in it

One checkout serves every session, so other sessions work beside yours:

- Write only under your branch's subdirectory and `carry-over/`.
- Commit there as the work goes, naming the paths you wrote: `git add <path>`, then `git commit -- <path>`, which leaves whatever another session has staged out of your commit.
- List environments and build output, such as `.venv/` and `target/`, in the workspace's `.gitignore`, so that `git add` leaves them out.
- Delete ignored files only when the person asks for the disk space back, and then with `git clean -fdX -- <subdirectory>`, which removes the ignored files under that one subdirectory and nothing else.

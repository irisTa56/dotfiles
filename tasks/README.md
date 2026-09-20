# Shared tasks

`tasks/` holds the repository-agnostic tasks this repository lends to others: `secrets:scan`, which gates a commit here too, and `shared-tasks:check` below.
The first half of this file is for a repository taking them, the second for whoever adds the next one.

## Taking them

A consuming repository picks them up with a [`task_config.includes`](https://mise.jdx.dev/tasks/task-configuration.html#task_config.includes) entry pointing here, and needs no `[tools]` of its own: a task that needs a tool mise can install declares it, and mise installs it for that task alone.
Beyond that the library assumes git and bash.

```toml
# mise.toml, in the consuming repository
[task_config]
includes = [
  "git::https://github.com/irisTa56/dotfiles.git//tasks?ref=<40-character commit sha>",
  "mise-tasks", # and whichever other default directories that repository uses
]
```

mise marks a `git::` include experimental, so the syntax and the caching below carry no compatibility promise and a consumer wired to several of these is worth keeping an eye on mise's releases for.
Naming `includes` replaces the default file-task directories rather than adding to them, so a repository that keeps its own tasks in one has to list it back.
Of two entries defining the same task the later wins, which is how a repository overrides a shared one: it puts its own file task under a directory listed after this include.
An inline `[tasks.<name>]` does not do that.
mise lays its fields over the shared file task and keeps the file, so `mise tasks ls` reports the inline description while `mise run` executes the shared script.

Pin `ref` to a commit, not a branch, and spell it in full.
mise [keys its clone cache on the repository URL and the ref alone, and reuses an existing clone without fetching](https://github.com/jdx/mise/blob/v2026.9.11/src/task/task_file_providers/remote_task_git.rs), so a branch ref stays at whatever it first resolved to; a new commit sha is a new key and clones afresh.
It recognises a sha only at 40 characters and treats anything shorter as a branch or tag name, which fails with "the remote didn't have any ref that matched" rather than with anything about its length.
That link is pinned to a release for the same reason the `ref` is: on `main` it would keep resolving while quietly ceasing to support the claim.

### `secrets:scan`

It reads the staged diff, so it belongs in a pre-commit hook.
A plain checkout stages nothing and it passes having scanned nothing, which is not a CI gate.

### `shared-tasks:check`

It says whether the tasks a pin brought in are the ones on `main`.
It answers for those tasks rather than for the commit carrying them, because `main` moves for reasons that never reach them.
It exits 1 when they differ and names both commits, without saying which is the older, since the pin may be ahead of `main` as well as behind it.
It exits 2 when it could not tell instead — no `MISE_TASK_DIR`, tasks that came from no clone or from another repository's, or an unreachable remote — and says which of those it was.
Run inside this repository it exits 0 and checks nothing, since its own working tree carries no pin.

Give it a CI step of its own and fail the build on anything but 0, so a failure costs a re-run and nothing half-done.
Of the four conditions behind exit 2 only the unreachable remote clears on a retry; the other three stand until someone fixes the include.

## Writing one

Each task is an executable file, and the directory it sits in is its namespace: `secrets/scan` is the task `secrets:scan`.
mise [loads every executable under a task directory](https://mise.jdx.dev/tasks/file-tasks.html), at whatever depth, so an executable placed here to be run by another task would become a task of its own in every consuming repository, under a name nobody chose.
A file that is not executable is not loaded either, with one exception, and a task can source one through `$MISE_TASK_DIR`.
Nothing here does, and a helper added that way would have to be added to every `find tasks` selector in `mise.toml` as well, since each of them picks the executables alone — the two `pre-commit` lints with, and the one `sh:format` formats with.
The exception is `.toml`: mise reads every one that is not a config file as a list of tasks, so a data file dropped in here becomes tasks named after its keys, or breaks task loading outright.
Keep such a file out of here.
[`task_config.excludes`](https://mise.jdx.dev/tasks/task-configuration.html#task_config.excludes) belongs to the config that declares the include, so naming it in this repository's `mise.toml` would clear it here and leave every consumer loading it.

A task here is a shell script: this repository hands every executable under this directory to shellcheck and `shfmt`, so one written in another language fails `pre-commit`, and goes on failing `sh:format` until every `find tasks` selector in `mise.toml` narrows.
Declare in the task's `#MISE` header whatever tool mise can install for it, and start no later comment line with `MISE`.
mise reads the comment block following the header as more of the header, so a line opening with `MISE_TASK_DIR` or `MISE_CACHE_DIR` is parsed as a usage spec and every run of the task prints a parse warning.
Writing the variable with its sigil, as `$MISE_TASK_DIR`, keeps the line out of that.
Neither mistake shows up in this repository's own `mise run pre-commit`: it runs one of these tasks, `[tools]` in `mise.toml` puts their tools on PATH whatever their headers say, and the usage-spec warning appears only when the task it belongs to runs.

A task runs in the consuming repository: its working directory is that repository's root, and the paths it names resolve there rather than here.
[`MISE_TASK_DIR`](https://mise.jdx.dev/tasks/#environment-variables-passed-to-tasks) is the one path that points back into this library, at the directory the task was loaded from — the working tree here, or a cached clone of this repository in a consumer.

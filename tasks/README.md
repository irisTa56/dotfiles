# Shared tasks

`tasks/` holds the repository-agnostic mise tasks this repository lends to others: `secrets:scan` and `secrets:push-scan`, which gate a commit and a push here too, and `shared-tasks:check` below.
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
Do not declare an inline `[tasks.<name>]` of the same name.
mise does not merge one with the shared file task: it keeps that under `<name>.sh` and gives the bare name to the declaration.
One carrying `run` then runs instead of the shared task, and one carrying only a `description` or a `dir` runs nothing at all and exits 0.
One carrying only `depends` runs its dependencies, reports finishing and exits 0 without ever reaching the shared task, which reads as a pass.
`mise tasks ls` shows both names, so checking there confirms the mistake rather than exposing it.

Pin `ref` to a commit on `main`, not a branch, and spell it in full.
For a sha mise clones the whole repository and checks the commit out, so a commit no branch reaches is not in the clone: a pin taken from a branch since deleted, or from a pull request this repository squash-merged, fails the next time the cache is cold rather than when it was written.
A failing include takes every task in the consuming repository with it, that repository's own included.
mise [keys its clone cache on the repository URL and the ref alone, and reuses an existing clone without fetching](https://github.com/jdx/mise/blob/v2026.9.11/src/task/task_file_providers/remote_task_git.rs), so a branch ref stays at whatever it first resolved to; a new commit sha is a new key and clones afresh.
It recognises a sha only at 40 characters and treats anything shorter as a branch or tag name, which fails with "the remote didn't have any ref that matched" rather than with anything about its length.
That link is pinned to a release for the same reason the `ref` is: on `main` it would keep resolving while quietly ceasing to support the claim.

### `secrets:scan`

It reads the staged diff, so it belongs in a pre-commit hook.
A plain checkout stages nothing and it passes having scanned nothing, which is not a CI gate.

### `secrets:push-scan`

It runs trufflehog over the commits a push is about to send on a branch, and fails the push on a hit.
`secrets:scan` gates each commit, but gitleaks' default rules miss a credential embedded in a connection string or a URL, and so does [GitHub's push protection](https://docs.github.com/en/code-security/secret-scanning/introduction/supported-secret-scanning-patterns), which covers provider tokens rather than the generic patterns such a credential falls under.

It reads the pushed refs from stdin and takes the remote's name from its first argument, so a pre-push hook is the only caller it works for.
mise writes one, resolving the shared hooks directory from a linked worktree as well:

```shell
mise generate git-pre-commit --hook pre-push --task secrets:push-scan --write
```

Which repositories are scanned is which repositories have that hook, so there is no list of them to keep anywhere.
The command moves an existing `pre-push` aside as `pre-push.old`, so where another tool manages the hooks, have that tool run `mise run secrets:push-scan` instead.
A push from an environment with no mise on PATH, an editor's Git UI say, fails on the hook rather than going out unscanned.

### `shared-tasks:check`

It says whether the pin is the tip of `main`.
It answers for the commit rather than for what the commit changed, so a `main` that moved without touching these tasks still asks for a bump — which is one line, and which keeps a consumer's copy of these instructions current as well as their copy of the tasks.
It exits 1 when the two differ and names both, without saying which is the older, since the pin may be ahead of `main` as well as behind it.
It exits 2 when it could not tell instead — run outside mise, tasks that are not in a repository git will read, or a remote it could not read `refs/heads/main` from — and says which of those it was.
Run inside this repository it exits 0 and checks nothing, since its own working tree carries no pin.

Give it a CI step of its own and fail the build on anything but 0, so a failure costs a re-run and nothing half-done.

## Writing one

Each task is an executable `*.sh`, and the directory it sits in is its namespace: `secrets/scan.sh` is the task `secrets:scan`.
Name it that way or the linting below never sees it, and mise will load it all the same.
mise [loads every executable under a task directory](https://mise.jdx.dev/tasks/file-tasks.html), at whatever depth, so an executable placed here to be run by another task would become a task of its own in every consuming repository, under a name nobody chose.
A file that is not executable is not loaded either, with one exception, and a task can source one through `$MISE_TASK_DIR`.
Nothing here does, and one named `*.sh` would be linted like a task, since that is what `mise.toml` selects on.
The exception is `.toml`: mise reads every one that is not a config file as a list of tasks, so a data file dropped in here becomes tasks named after its keys, or breaks task loading outright.
Keep such a file out of here.
[`task_config.excludes`](https://mise.jdx.dev/tasks/task-configuration.html#task_config.excludes) belongs to the config that declares the include, so naming it in this repository's `mise.toml` would clear it here and leave every consumer loading it.

A task here is a shell script, and its name says so: this repository hands every `*.sh` under this directory to shellcheck and `shfmt`, so a task in another language is named out of that and goes unchecked until the `find` in `mise.toml` reaches it.
Declare in the task's `#MISE` header whatever tool mise can install for it, and start no later comment line with `MISE`.
mise reads the comment block following the header as more of the header, so a line opening with `MISE_TASK_DIR` or `MISE_CACHE_DIR` is parsed as a usage spec and every run of the task prints a parse warning.
Writing the variable with its sigil, as `$MISE_TASK_DIR`, keeps the line out of that.
Neither mistake shows up in this repository's own `mise run pre-commit`: it runs one of these tasks, `[tools]` in `mise.toml` puts their tools on PATH whatever their headers say, and the usage-spec warning appears only when the task it belongs to runs.

A task runs in the consuming repository: its working directory is that repository's root, and the paths it names resolve there rather than here.
[`MISE_TASK_DIR`](https://mise.jdx.dev/tasks/#environment-variables-passed-to-tasks) is the one path that points back into this library — into the working tree here, or into a cached clone of this repository in a consumer.
It is the task's own namespace directory rather than the root of the library: `tasks/secrets` for `secrets:scan`, so a file two tasks share sits at `$MISE_TASK_DIR/..`.

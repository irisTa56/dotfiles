# Shared tasks

Tasks other repositories load through `task_config.includes`, as [the repository README](../README.md#shared-mise-tasks) describes for the consuming side, including what a consumer is assumed to have.
This file is for whoever adds the next one.

Each task is an executable file, and the directory it sits in is its namespace: `secrets/scan` is the task `secrets:scan`.
mise [loads every executable under a task directory](https://mise.jdx.dev/tasks/file-tasks.html), at whatever depth, so an executable placed here to be run by another task would become a task of its own in every consuming repository, under a name nobody chose.
A file that is not executable is not loaded, and a task can source one through `$MISE_TASK_DIR`.
Nothing here does, and a helper added that way would have to be added to what `pre-commit` lints as well, since that selects the executables alone.

Declare in the task's `#MISE` header whatever tool mise can install for it, and start no later comment line with `MISE`.
mise reads the comment block following the header as more of the header, so a line opening with `MISE_TASK_DIR` or `MISE_CACHE_DIR` is parsed as a usage spec and every run of the task prints a parse warning.

A task runs in the consuming repository: its working directory is that repository's root, and the paths it names resolve there rather than here.
[`MISE_TASK_DIR`](https://mise.jdx.dev/tasks/#environment-variables-passed-to-tasks) is the one path that points back into this library, at the directory the task was loaded from — the working tree here, or a cached clone of this repository in a consumer.

# Shared tasks

Tasks other repositories load through `task_config.includes`, as [the repository README](../README.md#shared-mise-tasks) describes for the consuming side.
This file is for whoever adds the next one.

Each task is an executable file, and the directory it sits in is its namespace: `secrets/scan` is the task `secrets:scan`.
mise loads every executable under this directory as a task, so a file placed here to be run by another task would become a task of its own in every consuming repository, under a name nobody chose.
A task that needs more than it can carry is therefore split into more tasks, not into a script beside them.

A task declares in its `#MISE` header whatever tool mise can install for it, which is why a consuming repository needs no `[tools]` entry of its own.
Beyond that the library assumes git and bash.

A task runs in the consuming repository: its working directory is that repository's root, and the paths it names resolve there rather than here.
`MISE_TASK_DIR` is the one path that points back into this library, at the directory the task was loaded from — the working tree here, or a cached clone of this repository in a consumer.

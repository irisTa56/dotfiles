#!/usr/bin/env bash
#MISE description="Fail where a tracked skill's body is past what a compaction re-attaches; --verbose prints each passing skill's length too"
# What it checks, and what each exit code means, is in tasks/README.md under
# "skills:size-check". The check itself is the dotfiles-tasks binary, built from rust/
# and released from this repository; how the limit is set and counted is in
# rust/src/size_check.rs. mise installs the release pinned below, where a #MISE tools=
# header would not: a job that sets MISE_TASK_RUN_AUTO_INSTALL=false installs no task's
# tools. A failed install, or an installation mise then cannot find, is no answer about
# the skills, so it exits 2 rather than with mise's own status. The binary runs from
# where mise installed it rather than through `mise x`, which leaves the tools a task's
# header declares off PATH.
tool='github:irisTa56/dotfiles[version_prefix=dotfiles-tasks-v,bin=dotfiles-tasks]@0.2.1' # x-release-please-version
mise install --quiet "$tool" || exit 2
dir=$(mise where "$tool") || exit 2
exec "$dir/dotfiles-tasks" size-check "$@"

#!/usr/bin/env bash
#MISE description="Check whether the upstream paths this repository pins have changed on their default branches"
#MISE alias="shared-tasks:check"
# What a pin is, and what each exit code means, is in tasks/README.md under
# "upstream:pin-check". The check itself is the dotfiles-tasks binary, built from rust/
# and released from this repository. mise installs the release pinned below, where a
# #MISE tools= header would not: a job that sets MISE_TASK_RUN_AUTO_INSTALL=false
# installs no task's tools. A failed install is no answer about the pins, so it exits 2
# rather than with mise's own status.
tool='github:irisTa56/dotfiles[version_prefix=dotfiles-tasks-v,bin=dotfiles-tasks]@0.0.0' # x-release-please-version
mise install --quiet "$tool" || exit 2
exec mise x "$tool" -- dotfiles-tasks pin-check "$@"

#!/usr/bin/env bash
#MISE description="Check whether the upstream paths this repository pins have changed on their default branches"
#MISE alias="shared-tasks:check"
# What a pin is, and what each exit code means, is in tasks/README.md under
# "upstream:pin-check". The check itself is the dotfiles-tasks binary, built from rust/
# and released from this repository. `mise x` installs the release the line below pins,
# where a #MISE tools= header would not: a job that sets MISE_TASK_RUN_AUTO_INSTALL=false
# installs no task's tools.
exec mise x 'github:irisTa56/dotfiles[version_prefix=dotfiles-tasks-v,bin=dotfiles-tasks]@0.0.0' -- dotfiles-tasks pin-check "$@" # x-release-please-version

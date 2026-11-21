#!/usr/bin/env bash
#MISE description="Check the links of staged files"
#MISE tools={ lychee = "latest" }
# What it checks is in tasks/README.md under "link-check:staged". The check itself is
# the dotfiles-tasks binary, built from rust/ and released from this repository, which
# runs the lychee the header above puts on PATH; how it picks the files is in
# rust/src/link_check_staged.rs. The release pinned below is installed here rather than
# in the header, so that a job that sets MISE_TASK_RUN_AUTO_INSTALL=false, which
# installs no task's tools, has only lychee to install itself. A failed install, or an
# installation mise then cannot find, checks no link, so it exits 2 rather than with
# mise's own status. The binary runs from where mise installed it rather than through
# `mise x`, which leaves the tools a task's header declares off PATH.
tool='github:irisTa56/dotfiles[version_prefix=dotfiles-tasks-v,bin=dotfiles-tasks]@0.8.0' # x-release-please-version
mise install --quiet "$tool" || exit 2
dir=$(mise where "$tool") || exit 2
exec "$dir/dotfiles-tasks" link-check-staged "$@"

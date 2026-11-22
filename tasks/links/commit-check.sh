#!/usr/bin/env bash
#MISE description="Check the links of staged files"
#MISE tools={ lychee = "latest" }
#MISE alias="link-check:staged"
# What it checks is in tasks/README.md under "links:commit-check", and why it answers
# to its former name as well, under "Renamed tasks". The check itself is
# the dotfiles-tasks binary, built from rust/ and released from this repository, which
# runs the lychee the header above puts on PATH; how it picks the files is in
# rust/src/links_commit_check.rs. The release pinned below is installed here rather than
# in the header, so that a job that sets MISE_TASK_RUN_AUTO_INSTALL=false, which
# installs no task's tools, has only lychee to install itself. A failed install, or an
# installation mise then cannot find, checks no link, so it exits 2 rather than with
# mise's own status. The binary runs from where mise installed it rather than through
# `mise x`, which leaves the tools a task's header declares off PATH.
# mise runs the consuming project's [hooks] postinstall on every install there, and
# that setup is not this task's to run, so the install skips mise's hooks.
tool='github:irisTa56/dotfiles[version_prefix=dotfiles-tasks-v,bin=dotfiles-tasks]@0.11.0' # x-release-please-version
MISE_NO_HOOKS=1 mise install --quiet "$tool" || exit 2
dir=$(mise where "$tool") || exit 2
exec "$dir/dotfiles-tasks" links-commit-check "$@"

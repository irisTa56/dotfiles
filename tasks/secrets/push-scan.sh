#!/usr/bin/env bash
#MISE description="Scan the commits a push is about to send for secrets"
#MISE tools={ trufflehog = "latest" }
# A pre-push hook's task, and nothing else: git names the remote in its first argument
# and feeds one line per pushed ref on stdin, both of which reach the binary as given.
# What it reaches is answered in tasks/README.md under "secrets:push-scan". The scan
# itself is the dotfiles-tasks binary, built from rust/ and released from this
# repository, which runs the trufflehog the header above puts on PATH; how it reads the
# push is in rust/src/secrets_push_scan.rs. The release pinned below is installed here
# rather than in the header, so that a job that sets MISE_TASK_RUN_AUTO_INSTALL=false,
# which installs no task's tools, has only trufflehog to install itself. A failed
# install, or an installation mise then cannot find, scans nothing, so it exits 2 rather
# than with mise's own status. The binary runs from where mise installed it rather than
# through `mise x`, which leaves the tools a task's header declares off PATH.
# mise runs the consuming project's [hooks] postinstall on every install there, and
# that setup is not this task's to run, so the install skips mise's hooks.
tool='github:irisTa56/dotfiles[version_prefix=dotfiles-tasks-v,bin=dotfiles-tasks]@0.10.0' # x-release-please-version
MISE_NO_HOOKS=1 mise install --quiet "$tool" || exit 2
dir=$(mise where "$tool") || exit 2
exec "$dir/dotfiles-tasks" secrets-push-scan "$@"

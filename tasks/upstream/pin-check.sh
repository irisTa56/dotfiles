#!/usr/bin/env bash
#MISE description="Check whether the upstream paths this repository pins have changed on their default branches"
#MISE tools={"github:irisTa56/dotfiles"={version="0.0.0", version_prefix="dotfiles-tasks-v", bin="dotfiles-tasks"}} # x-release-please-version
#MISE alias="shared-tasks:check"
# What a pin is, and what each exit code means, is in tasks/README.md under
# "upstream:pin-check". The check itself is the dotfiles-tasks binary, built from rust/
# and released from this repository; the header pins the release this task runs.
exec dotfiles-tasks pin-check "$@"

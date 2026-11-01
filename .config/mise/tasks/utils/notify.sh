#!/usr/bin/env bash
#MISE description="Post a macOS notification, its title stamped with the time it is posted"
#MISE quiet=true
#USAGE arg "<title>"
#USAGE arg "<body>"
set -euo pipefail

# Call it as `mise -C ~ run --skip-tools utils:notify`, so a notification does not
# depend on what it may be reporting:
# - `-C ~` leaves out the config of the repository the caller sits in, which can fail
#   to load, or define a task of the same name that would run instead.
# - `--skip-tools` stops mise installing a missing global tool first, which fails the
#   task when offline. This task needs none, and its own header cannot turn that off.

# The text reaches AppleScript as arguments, so none of it needs escaping.
osascript \
  -e 'on run argv' \
  -e 'display notification (item 2 of argv) with title (item 1 of argv) sound name "Glass"' \
  -e 'end run' \
  "$1 ($(date '+%m/%d %H:%M'))" "$2"

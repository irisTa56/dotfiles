#!/usr/bin/env bash
set -euo pipefail

# Posts a macOS notification, its title stamped with the time it is posted:
#   desktop-notify <title> <body>
#
# Not a mise task: `mise run` first loads the config of the directory it runs in and
# installs any missing tool, and a notification must not depend on what may be the
# trouble it reports. Linked into ~/.local/bin by [dotfiles] in mise.toml.

# The text reaches AppleScript as arguments, so none of it needs escaping.
osascript \
  -e 'on run argv' \
  -e 'display notification (item 2 of argv) with title (item 1 of argv) sound name "Glass"' \
  -e 'end run' \
  "$1 ($(date '+%m/%d %H:%M'))" "$2"

#!/usr/bin/env bash
set -euo pipefail

# Usage: notify_desktop <title> <body>
# Posts a macOS notification, its title stamped with the time it is posted.

# The text reaches AppleScript as arguments, so none of it needs escaping.
osascript \
  -e 'on run argv' \
  -e 'display notification (item 2 of argv) with title (item 1 of argv) sound name "Glass"' \
  -e 'end run' \
  "$1 ($(date '+%m/%d %H:%M'))" "$2"

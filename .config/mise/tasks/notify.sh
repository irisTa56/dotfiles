#!/usr/bin/env bash
#MISE description="Post a macOS notification, its title stamped with the time it is posted"
#MISE quiet=true
#USAGE arg "<title>"
#USAGE arg "<body>"
set -euo pipefail

# A caller that runs late, as a job missed during sleep does, posts a time other
# than the one it was scheduled for, and the stamp is what shows that.
# The text reaches AppleScript as arguments, so none of it needs escaping.
osascript \
  -e 'on run argv' \
  -e 'display notification (item 2 of argv) with title (item 1 of argv) sound name "Glass"' \
  -e 'end run' \
  "$1 ($(date '+%m/%d %H:%M'))" "$2"

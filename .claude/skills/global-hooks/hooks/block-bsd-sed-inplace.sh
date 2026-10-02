#!/bin/sh
# PreToolUse hook for Bash: block BSD-style `sed -i ''` where the sed on PATH is
# GNU sed. GNU sed's -i takes no separate suffix argument, so the empty string
# becomes the script and the real script a file name, or with -e the empty
# string becomes a file name; either way the call exits 2, and without -e it
# edits nothing. Exit 2 blocks the call and hands stderr back to Claude. Where
# sed is BSD sed the form is the right one, so the hook passes everything there.
# Only a bare `sed` is matched: a path such as /usr/bin/sed names its own sed,
# and a quote or `|` right before the word marks an argument to another command.
# The same text after a space inside a quoted argument is still caught.

sed --version >/dev/null 2>&1 || exit 0

pattern="(^|[[:space:];&(])sed +(-[a-zA-Z]+ +)*-i +(''|\"\")"

if jq -r '.tool_input.command // empty' | grep -qE "$pattern"; then
  echo "Blocked: sed on PATH here is GNU sed, whose -i takes no separate suffix argument, so the empty string after -i is read as the script or as a file name and the call fails. Write \`sed -i 's/.../.../' file\`, or use the Edit tool where the edit is not mechanical." >&2
  exit 2
fi

#!/bin/sh
# PreToolUse hook for Bash: ask the user before a curl or wget call that carries
# their email address or its local part. Agents put it into a User-Agent for
# APIs that ask callers to identify themselves, where a name alone would do. The
# hook asks rather than blocks because some services do require the address, and
# the command does not say which kind this is.
# Only a bare `curl` or `wget` with the address written out is matched, which
# covers the forms seen in use. A path such as /usr/bin/curl, a quote or a
# backtick right before the word, a request made from another program, and an
# address assembled from pieces all pass unasked.

email=$(git config --global user.email) || exit 0
local_part=${email%%@*}
[ -n "$local_part" ] || exit 0

command=$(jq -r '.tool_input.command // empty')
printf '%s' "$command" | grep -qE '(^|[[:space:];&(|])(curl|wget)[[:space:]]' || exit 0
printf '%s' "$command" | grep -qiF "$local_part" || exit 0

jq -n '{
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "ask",
    permissionDecisionReason: "This request carries the user'\''s email address or part of it. Allow it where the service requires the address or the user asked for it to be sent. Otherwise deny it: the caller can identify itself by a tool or project name alone, as in `-A project-research`."
  }
}'

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
# The cause goes at the head of the call's description, which the permission
# prompt shows as its heading; the reason below it is drawn as plain text, no
# more prominent than the description, and an approver can read past it.

email=$(git config --global user.email) || exit 0
local_part=${email%%@*}
[ -n "$local_part" ] || exit 0

input=$(cat)
command=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')
printf '%s' "$command" | grep -qE '(^|[[:space:];&(|])(curl|wget)[[:space:]]' || exit 0
printf '%s' "$command" | grep -qiF "$local_part" || exit 0

printf '%s' "$input" | jq '{
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "ask",
    permissionDecisionReason: "It carries the user'\''s email address: allow it only if the service requires the address; otherwise -A project-research identifies the caller.",
    updatedInput: (.tool_input + {
      description: ("Sends the user'\''s email address" + (if (.tool_input.description // "") == "" then "" else ": " + .tool_input.description end))
    })
  }
}'

#!/bin/bash
# Tests for hooks/redact_secrets.sh, run as Claude Code runs it: a process fed a
# PostToolUse event on stdin. They need gitleaks and jq on PATH, as the hook does.
#
# The fake secrets are assembled at run time, so this file holds none for the
# repository's own secret scans to find.

# shellcheck disable=SC2016 # the $names in single-quoted filters are jq's
set -uo pipefail

hook="$(cd "$(dirname "$0")/../hooks" && pwd)/redact_secrets.sh"
chars="Q7vXk2mPz9RtL4wNc8HbJ5yFd3GsA6uE1oVi0KqTnWjYeZr"
github_pat="ghp_${chars:0:36}"
google_access="ya29.${chars:3:40}"
gcp_secret="GOCSPX-${chars:5:28}"
gcp_refresh="1//0g${chars:1:44}"
rclone_pass="${chars:7:30}"
withheld="[output withheld: the gitleaks secret scan could not run]"

failures=0
out="" status=0 stderr=""

# Runs the hook on the event given as $1, with any further arguments set in its
# environment, leaving its stdout, exit status and stderr in the globals above.
run_hook() {
  local event="$1" errfile
  shift
  errfile="$(mktemp)"
  out="$(env "$@" "$hook" <<<"$event" 2>"$errfile")"
  status=$?
  stderr="$(cat "$errfile")"
  rm -f "$errfile"
}

# Passes when the jq filter $2 holds of the hook's stdout, with any further
# arguments passed on to jq.
expect() {
  local name="$1" filter="$2"
  shift 2
  if jq -e "$@" "$filter" <<<"$out" >/dev/null 2>&1; then
    echo "ok   $name"
  else
    echo "FAIL $name"
    echo "     status=$status stdout=${out:0:300} stderr=${stderr:0:300}"
    failures=$((failures + 1))
  fi
}

bash_event() {
  jq -n --arg stdout "$1" --arg stderr "$2" '{hook_event_name: "PostToolUse", tool_name: "Bash",
    tool_input: {command: "cat creds"},
    tool_response: {stdout: $stdout, stderr: $stderr, interrupted: false,
      isImage: false, noOutputExpected: false}}'
}

read_event() {
  jq -n --arg content "$1" '{hook_event_name: "PostToolUse", tool_name: "Read",
    tool_input: {file_path: "/tmp/creds.conf"},
    tool_response: {type: "text", file: {filePath: "/tmp/creds.conf", content: $content,
      numLines: 4, startLine: 1, totalLines: 4}}}'
}

updated='.hookSpecificOutput.updatedToolOutput'

# Bash output is redacted and keeps its shape.
event="$(bash_event "$github_pat
{\"refresh_token\": \"$gcp_refresh\"}
{\"pass\": \"$rclone_pass\"}" "token=$google_access")"
run_hook "$event"
expect "bash: exit 0" '$s == 0' --argjson s "$status"
expect "bash: stdout redacted" "$updated.stdout | contains(\$a) or contains(\$b) or contains(\$c) | not" \
  --arg a "$github_pat" --arg b "$gcp_refresh" --arg c "$rclone_pass"
expect "bash: stdout marked" "$updated.stdout | contains(\"[REDACTED by gitleaks: github-pat]\")"
expect "bash: stderr redacted by the custom rule" \
  "$updated.stderr == \"token=[REDACTED by gitleaks: google-oauth-access-token]\""
expect "bash: shape kept" "$updated | del(.stdout, .stderr) == (\$e.tool_response | del(.stdout, .stderr))" \
  --argjson e "$event"

# Read content is redacted and keeps its shape.
event="$(read_event "[drive]
type = drive
client_secret = $gcp_secret
pass = $rclone_pass
")"
run_hook "$event"
expect "read: exit 0" '$s == 0' --argjson s "$status"
expect "read: content redacted" "$updated.file.content | contains(\$a) or contains(\$b) | not" \
  --arg a "$gcp_secret" --arg b "$rclone_pass"
expect "read: rclone rule marked" \
  "$updated.file.content | contains(\"pass = [REDACTED by gitleaks: rclone-obscured-password]\")"
expect "read: other lines kept" "$updated.file.content | contains(\"type = drive\")"
expect "read: shape kept" "$updated | .file.content = \"\" | . == (\$e.tool_response | .file.content = \"\")" \
  --argjson e "$event"

# Clean output is left alone.
run_hook "$(bash_event "total 0
README.md" "")"
expect "clean: exit 0, no output" '$s == 0 and $o == ""' -n --argjson s "$status" --arg o "$out"

# A binary Read is left alone, without even running the scanner.
run_hook "$(jq -n --arg b "$github_pat" '{tool_name: "Read",
  tool_response: {type: "image", file: {base64: $b, type: "image/png"}}}')" \
  REDACT_SECRETS_GITLEAKS=/nonexistent/gitleaks
expect "image: exit 0, no output" '$s == 0 and $o == ""' -n --argjson s "$status" --arg o "$out"

# Code that assigns to a password-named variable is not an rclone secret.
run_hook "$(read_event 'password=DEFAULT_PASSWORD_FROM_SETTINGS
auth = {"password": get_password_from_keychain_service()}
check_password(password=hashed_user_supplied_value_x)
password = encrypted_database_password_reference
')"
expect "code: exit 0, no output" '$s == 0 and $o == ""' -n --argjson s "$status" --arg o "$out"

# Thousands of findings still redact: the report outgrows an argument's limit.
many="$(for i in $(seq 1000 4000); do echo "ghp_${chars:0:32}$i"; done)"
run_hook "$(bash_event "$many" "")"
expect "many: exit 0, all redacted" \
  "\$s == 0 and ($updated.stdout | (contains(\"ghp_\") | not) and (split(\"\n\") | length == 3001))" \
  --argjson s "$status"

# An encoded secret, which gitleaks reports decoded, withholds the output.
run_hook "$(bash_event "data: $(printf 'token: %s' "$github_pat" | base64)" "")"
expect "encoded: exit 2 with a reason" '$s == 2 and ($e | contains("encoded"))' \
  -n --argjson s "$status" --arg e "$stderr"
expect "encoded: output withheld" \
  "$updated.stdout == \"[output withheld: gitleaks found a secret it cannot redact in place]\""

# A scanner that is missing or fails withholds the output.
for scanner in /nonexistent/gitleaks false; do
  run_hook "$(read_event "pass = $rclone_pass")" REDACT_SECRETS_GITLEAKS="$scanner"
  expect "$scanner: exit 2 with a reason" '$s == 2 and ($e | contains("withheld"))' \
    --argjson s "$status" --arg e "$stderr"
  expect "$scanner: content withheld, type kept" \
    "$updated == {type: \"text\", file: {filePath: \$w, content: \$w, numLines: 4, startLine: 1, totalLines: 4}}" \
    --arg w "$withheld"
done

if ((failures > 0)); then
  echo "$failures failed"
  exit 1
fi

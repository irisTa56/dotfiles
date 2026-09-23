#!/bin/bash
# PostToolUse hook: replace the secrets gitleaks finds in a tool's output.
#
# Reads the hook event on stdin. Every string in tool_response is scanned in one
# gitleaks run, and each secret found is replaced wherever it appears, so the
# output keeps the tool's own shape, which Claude Code requires of a replacement.
#
# When gitleaks cannot run, the output is withheld rather than passed on, and
# the hook exits 2 so Claude is told why. Any other failure also exits 2, and
# then the output does pass on, unscanned, since there is no jq left to hide it.
set -euo pipefail
trap 'echo "redact_secrets: failed at line $LINENO; this output was not scanned" >&2; exit 2' ERR

hooks_dir="$(cd "$(dirname "$0")" && pwd)"
# Tests point this at a stand-in to exercise the failure path.
gitleaks="${REDACT_SECRETS_GITLEAKS:-gitleaks}"

# Rewrites every string in the value, except the discriminators the output
# schema checks: withholding those too would make Claude Code reject the
# replacement and pass the original output on.
rewrite='def rewrite(f):
  if type == "string" then f
  elif type == "object" then
    with_entries(if .key == "type" or .key == "cell_type" then . else .value |= rewrite(f) end)
  elif type == "array" then map(rewrite(f))
  else . end;'
respond='{hookSpecificOutput: {hookEventName: "PostToolUse", updatedToolOutput: .}}'

event="$(cat)"

# Read returns images and PDFs as base64 payloads, not text a secret could be read from.
text="$(jq -r '.tool_response
  | select(type != "object" or ((.type // "") | IN("image", "pdf") | not))
  | [.. | strings | select(. != "")] | join("\n")' <<<"$event")"
[[ -n $text ]] || exit 0

err="$(mktemp)"
trap 'rm -f "$err"' EXIT
if ! report="$(printf '%s' "$text" | "$gitleaks" stdin --config "$hooks_dir/gitleaks.toml" \
  --no-banner --exit-code 0 --report-format json --report-path - --log-level error 2>"$err")" ||
  ! jq -e 'type == "array"' <<<"$report" >/dev/null 2>&1; then
  jq "$rewrite .tool_response | rewrite(if . == \"\" then . else \"[output withheld: the gitleaks secret scan could not run]\" end) | $respond" <<<"$event"
  echo "redact_secrets: withheld this tool's output: $gitleaks failed: $(cat "$err")" >&2
  exit 2
fi

# Longest first, so a secret that contains another is replaced whole.
jq --argjson findings "$report" "$rewrite"'
  ([$findings[] | select((.Secret // "") != "") | {secret: .Secret, rule: .RuleID}]
    | unique_by(.secret) | sort_by(-(.secret | length))) as $secrets
  | select($secrets != [])
  | .tool_response
  | rewrite(reduce $secrets[] as $s (.; split($s.secret) | join("[REDACTED by gitleaks: \($s.rule)]")))
  | '"$respond" <<<"$event"

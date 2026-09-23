#!/bin/bash
# PostToolUse hook: replace the secrets gitleaks finds in a tool's output.
#
# Reads the hook event on stdin. The strings in tool_response are scanned
# together, and each secret found is replaced wherever it appears, so the output
# keeps the tool's own shape, which Claude Code requires of a replacement.
#
# When gitleaks cannot run, or finds a secret that cannot be replaced verbatim,
# the output is withheld rather than passed on, and the hook exits 2 so Claude
# is told why. Any other failure also exits 2, and then the output does pass
# on, unscanned, since there is no jq left to hide it.
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

# Replaces every string in the output with the marker $1, tells Claude why ($2),
# and exits 2.
withhold() {
  jq --arg marker "$1" "$rewrite"' .tool_response | rewrite(if . == "" then . else $marker end) | '"$respond" <<<"$event"
  echo "redact_secrets: withheld this tool's output: $2" >&2
  exit 2
}

# gitleaks reads its input 100,000 bytes at a time and, finding no blank line
# to cut at, cuts mid-line, missing a secret that straddles the cut
# (defaultBufferSize and readUntilSafeBoundary in its sources/). So the text is
# scanned in pieces of whole lines no longer than that, each from a file, which
# gitleaks reads in one go where a pipe would hand it less. The report goes
# through a file too: passed as an argument, a long one exceeds ARG_MAX.
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
printf '%s\n' "$text" | LC_ALL=C awk -v dir="$work" -v max=100000 '
  size > 0 && size + length($0) + 1 > max { close(piece); n++; size = 0 }
  { piece = sprintf("%s/piece.%05d", dir, n); size += length($0) + 1; print > piece }'
scanned=true
for piece in "$work"/piece.*; do
  "$gitleaks" stdin --config "$hooks_dir/gitleaks.toml" --no-banner --exit-code 0 \
    --report-format json --report-path - --log-level error <"$piece" >"$piece.json" 2>>"$work/err" &&
    jq -e 'type == "array"' "$piece.json" >/dev/null 2>&1 || scanned=false
done
report="$work/report.json"
if ! $scanned || ! jq -s 'add' "$work"/piece.*.json >"$report" 2>/dev/null; then
  withhold "[output withheld: the gitleaks secret scan could not run]" "$gitleaks failed: $(cat "$work/err")"
fi

# gitleaks decodes base64, hex and percent-encoding before matching, and then
# reports the decoded secret, which the output does not contain verbatim.
if jq -e --slurpfile report "$report" '[.tool_response | .. | strings] as $strings
  | any($report[0][]; any(.Tags[]?; startswith("decoded:"))
    or (.Secret as $secret | $strings | any(contains($secret)) | not))' <<<"$event" >/dev/null; then
  withhold "[output withheld: gitleaks found a secret it cannot redact in place]" \
    "gitleaks found an encoded secret, or one it could not locate in the output"
fi

# Longest first, so a secret that contains another is replaced whole.
jq --slurpfile report "$report" "$rewrite"'
  ([$report[0][] | select((.Secret // "") != "") | {secret: .Secret, rule: .RuleID}]
    | unique_by(.secret) | sort_by(-(.secret | length))) as $secrets
  | select($secrets != [])
  | .tool_response
  | rewrite(reduce $secrets[] as $s (.; split($s.secret) | join("[REDACTED by gitleaks: \($s.rule)]")))
  | '"$respond" <<<"$event"

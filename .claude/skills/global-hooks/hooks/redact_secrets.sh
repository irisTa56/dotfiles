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

# Rewrites every string in the value, except the `type` discriminator the
# output schema checks: withholding it too would make Claude Code reject the
# replacement and pass the original output on.
rewrite='def rewrite(f):
  if type == "string" then f
  elif type == "object" then
    with_entries(if .key == "type" then . else .value |= rewrite(f) end)
  elif type == "array" then map(rewrite(f))
  else . end;'
respond='{hookSpecificOutput: {hookEventName: "PostToolUse", updatedToolOutput: .}}'

event="$(cat)"

# Read returns images, PDFs and PDF pages rendered as images ("parts") as base64
# payloads, not text a secret could be read from.
text="$(jq -r '.tool_response
  | select(type != "object" or ((.type // "") | IN("image", "pdf", "parts") | not))
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
# written as files of whole lines no longer than that, which gitleaks reads in
# one go where a pipe would hand it less, and scanned with `gitleaks dir`. Each
# file repeats the last 16,000 bytes of lines before it, so a multi-line secret
# such as a private key still lies whole in one; replacing by value makes the
# repeat harmless.
#
# gitleaks also honours `gitleaks:allow` comments and a .gitleaksignore in the
# directory it scans or the one it is pointed to, both of which a project keeps
# for its own commit scan; the scan ignores the one, and finds none of the other
# in $work.
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir "$work/pieces"
printf '%s\n' "$text" | LC_ALL=C awk -v dir="$work/pieces" -v max=100000 -v overlap=16000 '
  { line[NR] = $0 }
  END {
    for (first = 1; first <= NR; first = next_first) {
      piece = sprintf("%s/piece.%05d", dir, n++)
      for (last = first; last <= NR && (last == first || size + length(line[last]) + 1 <= max); last++) {
        print line[last] > piece
        size += length(line[last]) + 1
      }
      close(piece)
      size = 0
      if (last > NR) break
      for (next_first = last; next_first - 1 > first && back + length(line[next_first - 1]) + 1 <= overlap; next_first--)
        back += length(line[next_first - 1]) + 1
      back = 0
    }
  }'

# The report goes to a file: passed as an argument, a long one exceeds ARG_MAX.
report="$work/report.json"
if ! "$gitleaks" dir "$work/pieces" --config "$hooks_dir/gitleaks.toml" --no-banner --exit-code 0 \
  --ignore-gitleaks-allow --gitleaks-ignore-path "$work" \
  --report-format json --report-path "$report" --log-level error 2>"$work/err" ||
  ! jq -e 'type == "array"' "$report" >/dev/null 2>&1; then
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

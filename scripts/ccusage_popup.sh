#!/usr/bin/env bash
# Render a ccusage report in a Quick Look popup (monospace, full table).
# Usage: ccusage_popup.sh <daily|weekly>
set -euo pipefail

granularity="${1:-}"
case "$granularity" in
# BSD date (macOS) uses -v; GNU date (coreutils) uses -d. Try BSD first since
# GNU date rejects -v outright (clean non-zero exit), then fall back to GNU.
daily) since="$(date -v-2w +%Y%m%d 2>/dev/null || date -d '2 weeks ago' +%Y%m%d)" ;;
weekly) since="$(date -v-2m +%Y%m%d 2>/dev/null || date -d '2 months ago' +%Y%m%d)" ;;
*)
  echo "usage: ${0##*/} <daily|weekly>" >&2
  exit 2
  ;;
esac

# Capture the report without ANSI colors so it embeds cleanly as plain text.
out="$(ccusage claude "$granularity" --since "$since" --no-color)"

# Quick Look renders HTML via WebKit, guaranteeing a monospace font and exact
# box-drawing alignment that a proportional AppleScript dialog would mangle.
# Use a positional template (not -t) so mktemp works on both GNU and BSD.
dir="$(mktemp -d "${TMPDIR:-/tmp}/ccusage.XXXXXX")"
trap 'rm -rf "$dir"' EXIT
html="$dir/report.html"
{
  printf '%s\n' '<meta charset="utf-8"><body style="margin:0;background:#1e1e1e">'
  printf '%s\n' '<pre style="font:13px ui-monospace,Menlo,monospace;color:#eee;padding:16px">'
  # Escape HTML-significant characters in the report body.
  printf '%s' "$out" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'
  printf '\n%s\n' '</pre>'
} >"$html"

# qlmanage -p blocks until the preview panel is dismissed (the trap cleans up on
# every exit path). It is noisy on stderr even on success, so silence it but
# still surface an outright render failure instead of doing nothing.
qlmanage -p "$html" >/dev/null 2>&1 || {
  echo "Quick Look preview failed" >&2
  exit 1
}

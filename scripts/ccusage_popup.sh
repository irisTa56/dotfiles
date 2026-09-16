#!/usr/bin/env bash
# Render a ccusage report in a Quick Look popup (monospace, full table).
# Usage: ccusage_popup.sh <daily|weekly|monthly> [--by-repo]
#   --by-repo shows cost per repository (ccusage_by_repo.sh) instead of the
#   ccusage table.
set -euo pipefail

usage() {
  echo "usage: ${0##*/} <daily|weekly|monthly> [--by-repo]" >&2
  exit 2
}

granularity="${1:-}"
case "$granularity" in
# BSD date (macOS) uses -v; GNU date (coreutils) uses -d. Try BSD first since
# GNU date rejects -v outright (clean non-zero exit), then fall back to GNU.
daily) since="$(date -v-2w +%Y%m%d 2>/dev/null || date -d '2 weeks ago' +%Y%m%d)" ;;
# Start on a Sunday (ccusage's default week start) so the oldest of the 9 weeks
# is not a partial one.
weekly) since="$(date -v-sun -v-8w +%Y%m%d 2>/dev/null || date -d "$(date +%w) days ago 8 weeks ago" +%Y%m%d)" ;;
# Start on the 1st so the oldest of the 12 months is not a partial one.
monthly) since="$(date -v1d -v-11m +%Y%m%d 2>/dev/null || date -d "$(date +%Y-%m-01) 11 months ago" +%Y%m%d)" ;;
*) usage ;;
esac

case "${2:-}" in
"") by_repo=false ;;
--by-repo) by_repo=true ;;
*) usage ;;
esac

# Capture the report without ANSI colors so it embeds cleanly as plain text.
if "$by_repo"; then
  out="$("$(dirname "$0")/ccusage_by_repo.sh" "$granularity" --since "$since")"
else
  # Without a terminal ccusage assumes a narrow width and truncates cells
  # (weekly dates render as "2026-07-…"); at 160 columns the table fits in full.
  out="$(COLUMNS=200 ccusage claude "$granularity" --since "$since" --no-color)"
fi

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

#!/usr/bin/env bash
set -euo pipefail

# Exports the K-Boat repo catalogue into a fresh temporary directory and splits a compact
# copy of it into chunks of 100 notes, one per judging subagent.
# Prints one JSON object: the directory, the chunk files, the note count and the
# anomalies kboat-note reported. Exits non-zero when the catalogue was not read.

if [[ -z ${KBOAT_REPO:-} ]]; then
  echo "KBOAT_REPO is not set: it names this machine's k-boat checkout (dotfiles README, Setting up K-Boat)." >&2
  exit 2
fi

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
dir=$(mktemp -d)

status=0
mise -C "$KBOAT_REPO" x -- kboat-note list --type repo \
  --field title --field url --field description --field topics --field summary \
  --field domain --field role --field archived --field last_commit --field status \
  >"$dir/catalog.json" || status=$?

if [[ $status -ne 0 ]]; then
  # Exit 1 from kboat-note still carries the report, whose anomalies say why Repos/ could not be read.
  jq -n --arg dir "$dir" --argjson status "$status" --slurpfile out "$dir/catalog.json" \
    '{dir: $dir, exit: $status, anomalies: ($out[0].anomalies // null)}' 2>/dev/null ||
    printf '{"dir": "%s", "exit": %d, "anomalies": null}\n' "$dir" "$status"
  exit "$status"
fi

jq -c -f "$here/compact.jq" "$dir/catalog.json" >"$dir/compact.jsonl"
(cd "$dir" && split -l 100 compact.jsonl chunk-)

jq -n --arg dir "$dir" --slurpfile out "$dir/catalog.json" \
  --args '{dir: $dir, notes: ($out[0].notes | length), anomalies: $out[0].anomalies, chunks: $ARGS.positional}' \
  "$dir"/chunk-*

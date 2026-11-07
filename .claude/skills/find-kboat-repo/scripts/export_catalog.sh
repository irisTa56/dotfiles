#!/usr/bin/env bash
set -euo pipefail

# Exports the K-Boat repo catalogue into a fresh temporary directory and splits a compact
# copy of it into chunks small enough for one cheap subagent each to read whole.
# Prints one JSON object: the directory, the chunk files, the note count and the
# anomalies kboat-note reported. Exits non-zero, after printing what it has, when
# kboat-note could not read Repos/.

kboat_repo=${KBOAT_REPO:-$HOME/Documents/_repos/k-boat}
chunk_lines=${CHUNK_LINES:-100}
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
dir=$(mktemp -d)

status=0
mise -C "$kboat_repo" x -- kboat-note list --type repo \
  --field title --field url --field description --field topics --field summary \
  --field domain --field role --field archived --field last_commit --field status \
  >"$dir/catalog.json" || status=$?

if [[ $status -ne 0 ]]; then
  # Exit 1 still carries the report, whose anomalies say why Repos/ could not be read.
  jq -n --arg dir "$dir" --argjson status "$status" --slurpfile out "$dir/catalog.json" \
    '{dir: $dir, exit: $status, anomalies: ($out[0].anomalies // null)}' 2>/dev/null ||
    printf '{"dir": "%s", "exit": %d, "anomalies": null}\n' "$dir" "$status"
  exit "$status"
fi

jq -c -f "$here/compact.jq" "$dir/catalog.json" >"$dir/compact.jsonl"
(cd "$dir" && split -l "$chunk_lines" compact.jsonl chunk-)

jq -n --arg dir "$dir" --slurpfile out "$dir/catalog.json" \
  --args '{dir: $dir, notes: ($out[0].notes | length), anomalies: $out[0].anomalies, chunks: $ARGS.positional}' \
  "$dir"/chunk-*

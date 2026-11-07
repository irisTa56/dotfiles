#!/usr/bin/env bash
set -euo pipefail

# Prints, one JSON object per line, the record of each given title from the full
# export export_catalog.sh wrote into <dir>. A title with no record prints nothing.
# Usage: lookup.sh <dir> <title>...

dir=$1
shift
jq -c '.notes[].frontmatter | select(.title | IN($ARGS.positional[]))
  | {title, url, summary, status, last_commit, archived}' "$dir/catalog.json" --args "$@"

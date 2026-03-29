#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SKILLS_DIR="$SCRIPT_DIR/skills"

while IFS= read -r line; do
	# Skip empty lines and comments
	[[ -z "$line" || "$line" == \#* ]] && continue

	# Extract skill name: use --skill value if present, otherwise repo name
	if [[ "$line" == *--skill* ]]; then
		skill_name="${line##*--skill }"
		skill_name="${skill_name%% *}"
		skill_name="${skill_name%"${skill_name##*[![:space:]]}"}"
	else
		skill_name="${line##*/}"
		skill_name="${skill_name%% *}"
		skill_name="${skill_name%"${skill_name##*[![:space:]]}"}"
	fi

	if [[ ! -d "$SKILLS_DIR/$skill_name" ]]; then
		echo "  $skill_name"
	fi
done <"$SCRIPT_DIR/skills.txt"

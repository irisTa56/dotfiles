#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SKILLS_DIR="$SCRIPT_DIR/skills"

check_list() {
	local label="$1" file="$2" field="$3"

	echo "=== $label ==="

	while IFS= read -r line; do
		# Skip empty lines and comments
		[[ -z "$line" || "$line" == \#* ]] && continue

		skill_name="$(echo "$line" | awk -v f="$field" '{print $f}' | sed 's|.*/||')"
		if [[ ! -d "$SKILLS_DIR/$skill_name" ]]; then
			echo "  $skill_name"
		fi
	done <"$file"
}

check_list "Smithery Registry" "$SCRIPT_DIR/smithery-skills.txt" 1
check_list "SkillPort (GitHub)" "$SCRIPT_DIR/skillport-skills.txt" 2

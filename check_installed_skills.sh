#!/usr/bin/env bash
set -euo pipefail

install=0
confirm=0
for arg in "$@"; do
	case "$arg" in
	--install) install=1 ;;
	--confirm) confirm=1 ;;
	--help)
		echo "Usage: $0 [--install] [--confirm]"
		echo "  Lists skills in skills.txt that are not yet installed."
		echo "  With --install, installs the missing ones via npx skills."
		echo "  With --confirm, runs npx skills without -y so each skill prompts for confirmation."
		exit 0
		;;
	*)
		echo "Unknown option: $arg" >&2
		exit 1
		;;
	esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SKILLS_DIR="$SCRIPT_DIR/.claude/skills"

while IFS= read -r line <&3; do
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
		if ((install)); then
			echo "Installing $skill_name..."
			if ((confirm)); then
				# shellcheck disable=SC2086
				npx skills add $line -a claude-code
			else
				# shellcheck disable=SC2086
				npx skills add $line -a claude-code -y
			fi
		else
			echo "  $skill_name"
		fi
	fi
done 3<"$SCRIPT_DIR/skills.txt"

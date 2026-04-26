#!/usr/bin/env bash
# Dump Claude Desktop's mcpServers section to dotfiles/mcpServers.json (SoT).
# Usage: mcp_dump.sh
set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
src="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
out="$script_dir/../mcpServers.json"

[ -f "$src" ] || {
	echo "claude_desktop_config.json not found: $src" >&2
	exit 1
}

jq '.mcpServers // {}' "$src" >"$out.tmp" && mv "$out.tmp" "$out"
echo "[ok] $src → $out"

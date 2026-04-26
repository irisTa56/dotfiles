#!/usr/bin/env bash
# Diff each MCP client's user-scope servers against dotfiles/mcpServers.json.
# Lines marked '- missing:' are in the dump but absent from the client.
# Lines marked '+ extra:'   are in the client but absent from the dump.
set -euo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
dump="$script_dir/../mcpServers.json"

[ -f "$dump" ] || {
  echo "dump not found: $dump (run 'mise run mcp:dump' first)" >&2
  exit 1
}

dump_keys=$(jq -r 'keys[]' "$dump" | sort)

# label | config path | jq path to server map
clients=(
  "Claude Desktop|$HOME/Library/Application Support/Claude/claude_desktop_config.json|.mcpServers"
  "Claude Code|$HOME/.claude.json|.mcpServers"
  "VS Code|$HOME/Library/Application Support/Code/User/mcp.json|.servers"
)

for entry in "${clients[@]}"; do
  IFS='|' read -r label cfg jq_path <<<"$entry"
  printf '\n%s (%s):\n' "$label" "${cfg/#$HOME/~}"
  if [ ! -f "$cfg" ]; then
    echo "  (file not found)"
    continue
  fi
  client_keys=$(jq -r "($jq_path // {}) | keys[]" "$cfg" 2>/dev/null | sort)
  out=$(comm -3 <(echo "$dump_keys") <(echo "$client_keys") |
    awk -F'\t' 'NF==1{print "  - missing: " $1} NF==2{print "  + extra:   " $2}')
  if [ -z "$out" ]; then
    echo "  (in sync)"
  else
    echo "$out"
  fi
done

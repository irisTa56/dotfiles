#!/usr/bin/env bash
# Print Claude Code cost per repository as a period-by-repository table.
# Usage: ccusage_by_repo.sh <daily|weekly|monthly> [ccusage daily options...]
#   e.g. ccusage_by_repo.sh weekly --since 20260801 --offline
#
# ccusage reports usage per Claude Code project, whose name is the session's
# cwd with every non-alphanumeric character replaced by "-". That mapping is
# lossy ("my-repo" and "my/repo" collide), so each project is resolved through
# the real cwd recorded in its session logs instead of by parsing the name:
#   - a worktree under <repo>/.claude/worktrees/ counts toward <repo>;
#   - a session started in a Claude scratchpad (/private/tmp/claude-<uid>/
#     <parent project>/<session>/scratchpad/...) counts toward the repository
#     of the parent project that launched it;
#   - anything that is not a git repository counts toward "other".
# Weeks start on Sunday, matching `ccusage claude weekly`'s default.
set -euo pipefail

granularity="${1:-}"
case "$granularity" in
daily) period_header=Date ;;
weekly) period_header=Week ;;
monthly) period_header=Month ;;
*)
  echo "usage: ${0##*/} <daily|weekly|monthly> [ccusage daily options...]" >&2
  exit 2
  ;;
esac
shift

# ccusage reads the same locations: CLAUDE_CONFIG_DIR (comma-separated) when
# set, otherwise both the XDG and the legacy directory.
if [[ -n "${CLAUDE_CONFIG_DIR:-}" ]]; then
  IFS=, read -r -a config_dirs <<<"$CLAUDE_CONFIG_DIR"
else
  config_dirs=("$HOME/.config/claude" "$HOME/.claude")
fi

# Print the cwd recorded in the first session log of a project that has one.
project_cwd() {
  local root f line
  for root in "${config_dirs[@]}"; do
    for f in "$root/projects/$1"/*.jsonl; do
      [[ -f "$f" ]] || continue
      line="$(grep -m1 -o '"cwd":"[^"]*"' "$f")" || continue
      line="${line#\"cwd\":\"}"
      printf '%s\n' "${line%\"}"
      return 0
    done
  done
  return 1
}

# Print the repository label a cwd counts toward.
repo_label() {
  local dir="$1" parent_cwd common top
  if [[ "$dir" =~ ^(/private)?/tmp/claude-[0-9]+/([^/]+)/ ]]; then
    # A parent's name is a strict substring of this path, so recursion ends.
    if parent_cwd="$(project_cwd "${BASH_REMATCH[2]}")"; then
      repo_label "$parent_cwd"
    else
      echo other
    fi
    return
  fi
  dir="${dir%%/.claude/worktrees/*}"
  if [[ ! -d "$dir" ]]; then
    # A deleted temporary directory is not a repository; any other missing
    # directory is most likely a repository that has since been moved.
    case "$dir" in
    /tmp/* | /private/* | /var/*) echo other ;;
    *) basename "$dir" ;;
    esac
    return
  fi
  if ! common="$(git -C "$dir" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)"; then
    echo other
    return
  fi
  if [[ "$common" == */.git ]]; then
    # Covers the main checkout, its subdirectories, and any linked worktree.
    basename "${common%/.git}"
  else
    top="$(git -C "$dir" rev-parse --show-toplevel)"
    basename "$top"
  fi
}

usage_json="$(ccusage claude daily --instances --json "$@")"

labels=""
while IFS= read -r project; do
  if cwd="$(project_cwd "$project")"; then
    label="$(repo_label "$cwd")"
  else
    label=other
  fi
  labels+="$project"$'\t'"$label"$'\n'
done < <(jq -r '.projects // {} | keys[]' <<<"$usage_json")

printf 'Claude Code cost (USD) by repository - %s\n\n' "$granularity"

jq -r --arg g "$granularity" --arg header "$period_header" --arg labels "$labels" '
  def bucket:
    if $g == "daily" then .
    elif $g == "monthly" then .[0:7]
    else
      (strptime("%Y-%m-%d") | mktime) as $t
      | $t - ($t | gmtime | .[6]) * 86400 | strftime("%Y-%m-%d")
    end;
  def money:
    (. * 100 | round) as $c
    | ($c / 100 | floor | tostring | [scan("\\d{1,3}(?=(?:\\d{3})*$)")] | join(",")) as $int
    | ($c % 100 | tostring | if length < 2 then "0" + . else . end) as $frac
    | "$\($int).\($frac)";
  def pad($n; $left):
    ([range(0; $n - length)] | map(" ") | join("")) as $fill
    | if $left then . + $fill else $fill + . end;

  ($labels | split("\n") | map(select(length > 0) | split("\t") | {(.[0]): .[1]}) | add // {}) as $label_of
  | [.projects // {} | to_entries[] | ($label_of[.key] // "other") as $repo
      | .value[] | {repo: $repo, period: (.date | bucket), cost: .totalCost}] as $rows
  | if $rows == [] then "No usage data." else
      ($rows | reduce .[] as $r ({}; .[$r.period][$r.repo] += $r.cost)) as $cell
      | ($rows | reduce .[] as $r ({}; .[$r.repo] += $r.cost)) as $repo_total
      | ($repo_total | to_entries | sort_by(.key == "other", -.value) | map(.key)) as $repos
      | ([[$header] + $repos + ["Total"]]
         + ($cell | keys | map(. as $p
             | [$p] + ($repos | map($cell[$p][.] | if . == null then "" else money end))
               + [$cell[$p] | add | money]))
         + [["Total"] + ($repos | map($repo_total[.] | money)) + [$repo_total | add | money]]
        ) as $table
      | ([range(0; $table[0] | length)] | map(. as $i | $table | map(.[$i] | length) | max)) as $width
      | def line: [to_entries[] | .key as $i | .value | pad($width[$i]; $i == 0)] | join("  ");
        def rule: $width | map([range(0; .)] | map("-") | join("")) | join("  ");
        ([$table[0] | line, rule] + ($table[1:-1] | map(line)) + [rule, ($table[-1] | line)])
        | join("\n")
    end
' <<<"$usage_json"

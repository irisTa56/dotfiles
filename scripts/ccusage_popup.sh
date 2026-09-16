#!/usr/bin/env bash
# Render Claude Code usage in a Quick Look popup as three tables per period:
# token usage with the cache hit rate, cost by model, and cost by repository.
# Usage: ccusage_popup.sh <daily|weekly|monthly>
#
# All three come from a single `ccusage claude daily --instances --json` run,
# bucketed here into days, weeks (starting on Sunday, as `ccusage claude
# weekly` does by default) or months.
#
# ccusage names each project after the session's cwd with every
# non-alphanumeric character replaced by "-". That mapping is lossy ("my-repo"
# and "my/repo" collide), so each project is resolved through the real cwd
# recorded in its session logs instead of by parsing the name:
#   - a worktree under <repo>/.claude/worktrees/ counts toward <repo>;
#   - a session started in a Claude scratchpad (/private/tmp/claude-<uid>/
#     <parent project>/<session>/scratchpad/...) counts toward the repository
#     of the parent project that launched it;
#   - anything that is not a git repository counts toward "other".
set -euo pipefail

granularity="${1:-}"
case "$granularity" in
# BSD date (macOS) uses -v; GNU date (coreutils) uses -d. Try BSD first since
# GNU date rejects -v outright (clean non-zero exit), then fall back to GNU.
daily)
  period_header=Date
  since="$(date -v-2w +%Y%m%d 2>/dev/null || date -d '2 weeks ago' +%Y%m%d)"
  ;;
weekly)
  period_header=Week
  # Start on a Sunday so the oldest of the 9 weeks is not a partial one.
  since="$(date -v-sun -v-8w +%Y%m%d 2>/dev/null || date -d "$(date +%w) days ago 8 weeks ago" +%Y%m%d)"
  ;;
monthly)
  period_header=Month
  # Start on the 1st so the oldest of the 12 months is not a partial one.
  since="$(date -v1d -v-11m +%Y%m%d 2>/dev/null || date -d "$(date +%Y-%m-01) 11 months ago" +%Y%m%d)"
  ;;
*)
  echo "usage: ${0##*/} <daily|weekly|monthly>" >&2
  exit 2
  ;;
esac

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

usage_json="$(ccusage claude daily --instances --json --since "$since")"

labels=""
while IFS= read -r project; do
  if cwd="$(project_cwd "$project")"; then
    label="$(repo_label "$cwd")"
  else
    label=other
  fi
  labels+="$project"$'\t'"$label"$'\n'
done < <(jq -r '.projects // {} | keys[]' <<<"$usage_json")

out="$(jq -r --arg g "$granularity" --arg header "$period_header" --arg labels "$labels" '
  def bucket:
    if $g == "daily" then .
    elif $g == "monthly" then .[0:7]
    else
      (strptime("%Y-%m-%d") | mktime) as $t
      | $t - ($t | gmtime | .[6]) * 86400 | strftime("%Y-%m-%d")
    end;
  def commas: tostring | [scan("\\d{1,3}(?=(?:\\d{3})*$)")] | join(",");
  def money:
    (. * 100 | round) as $c
    | ($c % 100 | tostring | if length < 2 then "0" + . else . end) as $frac
    | "$\($c / 100 | floor | commas).\($frac)";
  def percent: (. * 1000 | round) as $p | "\($p / 10 | floor).\($p % 10)%";
  def pad($n; $left):
    ([range(0; $n - length)] | map(" ") | join("")) as $fill
    | if $left then . + $fill else $fill + . end;

  # A header row, body rows and a total row, as arrays of cell strings.
  def render($title):
    . as $table
    | ([range(0; $table[0] | length)] | map(. as $i | $table | map(.[$i] | length) | max)) as $width
    | def line: [to_entries[] | .key as $i | .value | pad($width[$i]; $i == 0)] | join("  ");
      def rule: $width | map([range(0; .)] | map("-") | join("")) | join("  ");
      [$title, "", ($table[0] | line), rule] + ($table[1:-1] | map(line)) + [rule, ($table[-1] | line)]
      | join("\n");

  # [{period, key, cost}] as a period-by-key cost table, keys ordered by their
  # total cost with "other" last.
  def cost_pivot($title):
    . as $rows
    | (reduce $rows[] as $r ({}; .[$r.period][$r.key] += $r.cost)) as $cell
    | (reduce $rows[] as $r ({}; .[$r.key] += $r.cost)) as $key_total
    | ($key_total | to_entries | sort_by(.key == "other", -.value) | map(.key)) as $keys
    | [[$header] + $keys + ["Total"]]
      + ($cell | keys | map(. as $p
          | [$p] + ($keys | map($cell[$p][.] | if . == null then "" else money end))
            + [$cell[$p] | add | money]))
      + [["Total"] + ($keys | map($key_total[.] | money)) + [$key_total | add | money]]
    | render($title);

  # Cache reads over all input tokens; the three input counts are disjoint.
  def usage_row($name):
    (.input + .cacheCreate + .cacheRead) as $all_input
    | [$name, (.input | commas), (.output | commas), (.cacheCreate | commas),
       (.cacheRead | commas), (.tokens | commas),
       (if $all_input == 0 then 0 else .cacheRead / $all_input end | percent),
       (.cost | money)];
  def add_usage($e):
    .input += $e.inputTokens | .output += $e.outputTokens
    | .cacheCreate += $e.cacheCreationTokens | .cacheRead += $e.cacheReadTokens
    | .tokens += $e.totalTokens | .cost += $e.totalCost;

  ($labels | split("\n") | map(select(length > 0) | split("\t") | {(.[0]): .[1]}) | add // {}) as $label_of
  | [.projects // {} | to_entries[] | ($label_of[.key] // "other") as $repo
      | .value[] | . + {repo: $repo, period: (.date | bucket)}] as $entries
  | if $entries == [] then "No usage data." else
      (reduce $entries[] as $e ({}; .[$e.period] |= add_usage($e))) as $usage
      | (reduce $entries[] as $e ({}; add_usage($e))) as $usage_total
      | ([[$header, "Input", "Output", "Cache Create", "Cache Read", "Total Tokens", "Cache Hit", "Cost (USD)"]]
         + ($usage | keys | map(. as $p | $usage[$p] | usage_row($p)))
         + [$usage_total | usage_row("Total")]
         | render("Claude Code usage - \($g)")),
        "",
        ([$entries[] | .period as $p | .modelBreakdowns[]
           | {period: $p, key: (.modelName | sub("^claude-"; "") | sub("-[0-9]{8}$"; "")), cost}]
         | cost_pivot("Cost (USD) by model - \($g)")),
        "",
        ([$entries[] | {period, key: .repo, cost: .totalCost}]
         | cost_pivot("Cost (USD) by repository - \($g)"))
    end
' <<<"$usage_json")"

# Quick Look renders HTML via WebKit, guaranteeing a monospace font and exact
# column alignment that a proportional AppleScript dialog would mangle.
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

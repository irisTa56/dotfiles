#!/usr/bin/env elixir

defmodule CcusagePopup do
  @usage """
  Usage: ccusage_popup.exs <daily|weekly|monthly>

  Shows Claude Code usage in a Quick Look popup as three tables per period:
  token usage with the cache hit rate, cost by model, and cost by repository.
  Periods cover the last 2 weeks, the last 9 weeks starting on a Sunday (as
  `ccusage claude weekly` does by default), or the last 12 months.
  """

  # ccusage names each project after the session's cwd with every
  # non-alphanumeric character replaced by "-". That mapping is lossy ("my-repo"
  # and "my/repo" collide), so each project is resolved through the real cwd
  # recorded in its session logs instead of by parsing the name:
  #   - a worktree under <repo>/.claude/worktrees/ counts toward <repo>;
  #   - a session started in a Claude scratchpad (/private/tmp/claude-<uid>/
  #     <parent project>/<session>/scratchpad/...) counts toward the repository
  #     of the parent project that launched it;
  #   - anything that is not a git repository counts toward "other".
  @scratchpad ~r{^(?:/private)?/tmp/claude-\d+/([^/]+)/}

  def main(argv) do
    case argv do
      ["daily"] ->
        run(:daily, "Date", Date.add(today(), -14))

      ["weekly"] ->
        run(:weekly, "Week", today() |> week_start() |> Date.add(-56))

      ["monthly"] ->
        run(:monthly, "Month", today() |> Date.beginning_of_month() |> Date.shift(month: -11))

      _ ->
        die(@usage, 2)
    end
  end

  defp run(granularity, period_header, since) do
    projects =
      ccusage!([
        "claude",
        "daily",
        "--instances",
        "--json",
        "--since",
        Calendar.strftime(since, "%Y%m%d")
      ])
      |> JSON.decode!()
      |> Map.get("projects", %{})

    entries =
      for {project, days} <- projects,
          repo = project |> project_cwd() |> repo_label(),
          day <- days do
        %{
          period: bucket(day["date"], granularity),
          repo: repo,
          models: day["modelBreakdowns"],
          usage: %{
            input: day["inputTokens"],
            output: day["outputTokens"],
            cache_create: day["cacheCreationTokens"],
            cache_read: day["cacheReadTokens"],
            tokens: day["totalTokens"],
            cost: day["totalCost"]
          }
        }
      end

    report =
      if entries == [] do
        "No usage data."
      else
        name = Atom.to_string(granularity)

        model_costs =
          for entry <- entries, model <- entry.models do
            {entry.period, model_name(model["modelName"]), model["cost"]}
          end

        repo_costs = Enum.map(entries, &{&1.period, &1.repo, &1.usage.cost})

        [
          usage_table("Claude Code usage - #{name}", period_header, entries),
          cost_table("Cost (USD) by model - #{name}", period_header, model_costs),
          cost_table("Cost (USD) by repository - #{name}", period_header, repo_costs)
        ]
        |> Enum.join("\n\n")
      end

    popup(report)
  end

  # The local date, which is also the date ccusage groups usage by.
  defp today do
    {date, _time} = :calendar.local_time()
    Date.from_erl!(date)
  end

  defp week_start(date), do: Date.beginning_of_week(date, :sunday)

  defp bucket(date, :daily), do: date

  defp bucket(date, :weekly),
    do: date |> Date.from_iso8601!() |> week_start() |> Date.to_iso8601()

  defp bucket(date, :monthly), do: String.slice(date, 0, 7)

  defp model_name(name) do
    name |> String.replace_prefix("claude-", "") |> String.replace(~r/-\d{8}$/, "")
  end

  # ccusage reads the same locations: CLAUDE_CONFIG_DIR (comma-separated) when
  # set, otherwise both the XDG and the legacy directory.
  defp config_dirs do
    case System.get_env("CLAUDE_CONFIG_DIR", "") do
      "" -> Enum.map([".config/claude", ".claude"], &Path.join(System.user_home!(), &1))
      dirs -> String.split(dirs, ",", trim: true)
    end
  end

  # The cwd recorded in the first session log of a project that has one.
  defp project_cwd(project) do
    for root <- config_dirs(),
        log <- Path.wildcard(Path.join([root, "projects", project, "*.jsonl"])) do
      log
    end
    |> Enum.find_value(fn log -> log |> File.stream!() |> Enum.find_value(&line_cwd/1) end)
  end

  defp line_cwd(line) do
    case JSON.decode(line) do
      {:ok, %{"cwd" => cwd}} when is_binary(cwd) -> cwd
      _ -> nil
    end
  end

  # The repository label a cwd counts toward.
  defp repo_label(nil), do: "other"

  defp repo_label(cwd) do
    case Regex.run(@scratchpad, cwd) do
      # A parent's name is a strict part of this path, so recursion ends.
      [_, parent] -> parent |> project_cwd() |> repo_label()
      nil -> cwd |> String.split("/.claude/worktrees/", parts: 2) |> hd() |> checkout_label()
    end
  end

  defp checkout_label(dir) do
    cond do
      # A deleted temporary directory is not a repository; any other missing
      # directory is most likely a repository that has since been moved.
      not File.dir?(dir) ->
        if String.starts_with?(dir, ["/tmp/", "/private/", "/var/"]),
          do: "other",
          else: Path.basename(dir)

      # The common dir is <repo>/.git for the main checkout, its subdirectories
      # and any linked worktree alike.
      common = git(dir, ["--path-format=absolute", "--git-common-dir"]) ->
        if String.ends_with?(common, "/.git"),
          do: common |> Path.dirname() |> Path.basename(),
          else: dir |> git(["--show-toplevel"]) |> Path.basename()

      true ->
        "other"
    end
  end

  defp git(dir, args) do
    case System.cmd("git", ["-C", dir, "rev-parse" | args], stderr_to_stdout: true) do
      {out, 0} -> String.trim(out)
      _ -> nil
    end
  end

  defp usage_table(title, period_header, entries) do
    by_period =
      entries
      |> Enum.group_by(& &1.period, & &1.usage)
      |> Enum.map(fn {period, usages} -> usage_row(period, sum_usage(usages)) end)
      |> Enum.sort()

    total = usage_row("Total", entries |> Enum.map(& &1.usage) |> sum_usage())

    header = [
      period_header,
      "Input",
      "Output",
      "Cache Create",
      "Cache Read",
      "Total Tokens",
      "Cache Hit",
      "Cost (USD)"
    ]

    render(title, [header | by_period] ++ [total])
  end

  defp sum_usage(usages), do: Enum.reduce(usages, &Map.merge(&1, &2, fn _key, a, b -> a + b end))

  # The three input counts are disjoint, so cache reads over their sum is the
  # share of all input tokens served from the cache.
  defp usage_row(name, usage) do
    all_input = usage.input + usage.cache_create + usage.cache_read
    hit = if all_input == 0, do: 0, else: usage.cache_read / all_input

    [
      name,
      commas(usage.input),
      commas(usage.output),
      commas(usage.cache_create),
      commas(usage.cache_read),
      commas(usage.tokens),
      percent(hit),
      money(usage.cost)
    ]
  end

  # {period, key, cost} triples as a period-by-key table, keys ordered by their
  # total cost with "other" last.
  defp cost_table(title, period_header, costs) do
    cells = Enum.group_by(costs, fn {period, key, _} -> {period, key} end, &elem(&1, 2))

    key_totals =
      costs
      |> Enum.group_by(&elem(&1, 1), &elem(&1, 2))
      |> Map.new(fn {key, cs} -> {key, Enum.sum(cs)} end)

    keys =
      key_totals
      |> Enum.sort_by(fn {key, cost} -> {key == "other", -cost} end)
      |> Enum.map(&elem(&1, 0))

    rows =
      for period <- costs |> Enum.map(&elem(&1, 0)) |> Enum.uniq() |> Enum.sort() do
        row_cells = Enum.map(keys, &Map.get(cells, {period, &1}))
        row_total = row_cells |> Enum.reject(&is_nil/1) |> Enum.map(&Enum.sum/1) |> Enum.sum()

        [period | Enum.map(row_cells, &if(&1, do: money(Enum.sum(&1)), else: ""))] ++
          [money(row_total)]
      end

    total =
      ["Total" | Enum.map(keys, &money(key_totals[&1]))] ++
        [money(Enum.sum(Map.values(key_totals)))]

    render(title, [[period_header | keys] ++ ["Total"] | rows] ++ [total])
  end

  # Rows are a header, the body and a total, each a list of cell strings. The
  # first column is left-aligned and the rest, all numbers, right-aligned.
  defp render(title, [header | rest] = rows) do
    {body, [total]} = Enum.split(rest, -1)
    widths = Enum.zip_with(rows, fn col -> col |> Enum.map(&String.length/1) |> Enum.max() end)
    rule = Enum.map_join(widths, "  ", &String.duplicate("-", &1))

    line = fn [first | others] ->
      [
        String.pad_trailing(first, hd(widths))
        | Enum.zip_with(others, tl(widths), &String.pad_leading/2)
      ]
      |> Enum.join("  ")
    end

    Enum.join(
      [title, "", line.(header), rule] ++ Enum.map(body, line) ++ [rule, line.(total)],
      "\n"
    )
  end

  defp commas(n), do: n |> Integer.to_string() |> String.replace(~r/\B(?=(\d{3})+$)/, ",")

  defp money(amount) do
    cents = round(amount * 100)

    "$#{commas(div(cents, 100))}.#{cents |> rem(100) |> Integer.to_string() |> String.pad_leading(2, "0")}"
  end

  defp percent(ratio) do
    permille = round(ratio * 1000)
    "#{div(permille, 10)}.#{rem(permille, 10)}%"
  end

  # Quick Look renders HTML via WebKit, guaranteeing a monospace font and exact
  # column alignment that a proportional AppleScript dialog would mangle.
  defp popup(report) do
    dir = Path.join(System.tmp_dir!(), "ccusage.#{System.unique_integer([:positive])}")
    File.mkdir_p!(dir)
    html = Path.join(dir, "report.html")

    body =
      report
      |> String.replace("&", "&amp;")
      |> String.replace("<", "&lt;")
      |> String.replace(">", "&gt;")

    File.write!(html, """
    <meta charset="utf-8"><body style="margin:0;background:#1e1e1e">
    <pre style="font:13px ui-monospace,Menlo,monospace;color:#eee;padding:16px">
    #{body}
    </pre>
    """)

    # qlmanage -p blocks until the preview panel is dismissed. It is noisy on
    # stderr even on success, so its output is dropped, but an outright render
    # failure still surfaces instead of doing nothing.
    {_out, status} =
      try do
        System.cmd("qlmanage", ["-p", html], stderr_to_stdout: true)
      after
        File.rm_rf!(dir)
      end

    if status != 0, do: die("Quick Look preview failed", 1)
  end

  defp ccusage!(args) do
    case System.cmd("ccusage", args) do
      {out, 0} -> out
      {_out, code} -> die("ccusage #{Enum.join(args, " ")} failed (exit #{code})", code)
    end
  end

  defp die(message, code) do
    IO.puts(:stderr, message)
    System.halt(code)
  end
end

CcusagePopup.main(System.argv())

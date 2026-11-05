defmodule Dotfiles.MiseMultiUpgrade do
  @usage """
  Usage: mise run mise:multi-upgrade [--dry-run] [--include-worktrees] [--include-tmpdirs]

  Shows, for each directory with a tracked mise config, the tools `mise upgrade`
  there would upgrade, with the age of each version, and asks whether to upgrade
  it. Once every directory is answered, runs `mise upgrade` in the accepted ones,
  keeping each version request as it is. The global config counts as a directory
  of its own, upgraded with `mise upgrade --global`; a project is upgraded with
  `--no-global`, so a global tool is upgraded once.

    --dry-run            show every directory's table, and ask and upgrade nothing
    --include-worktrees  also process linked git worktrees, such as agents' worktrees
    --include-tmpdirs    also process configs under a temporary directory
    --help, -h           print this message; through the task, `-- --help`, since
                         mise answers a bare `--help` itself

  Latest is the newest version matching the request, which the upgrade installs;
  Bump Latest is the newest version overall, reachable only by changing the
  request, and is shown for information. A `-` marks nothing to report; a `?`
  marks a Released cell whose age could not be determined.
  """

  @switches [
    dry_run: :boolean,
    include_worktrees: :boolean,
    include_tmpdirs: :boolean,
    help: :boolean
  ]

  @headers [
    "Tool",
    "Requested",
    "Current",
    "Released",
    "Latest",
    "Released",
    "Bump Latest",
    "Released"
  ]

  # Returns the exit status rather than halting, so the run ends in one place.
  def main(argv) do
    case OptionParser.parse(argv, strict: @switches, aliases: [h: :help]) do
      {opts, [], []} ->
        opts = Keyword.merge(Enum.map(@switches, &{elem(&1, 0), false}), opts)
        if opts[:help], do: usage(), else: run(opts)

      _ ->
        die(@usage, 2)
    end
  catch
    {:die, message, code} ->
      IO.puts(:stderr, message)
      code
  end

  defp usage do
    IO.puts(@usage)
    0
  end

  # A reader that stops early — `| head`, or quitting the pager — closes the pipe
  # under us, and the next write raises. Left to propagate it exits non-zero and
  # dumps several megabytes of VM state into whatever directory the report was
  # run from, so the report is over rather than failed. Only the `--dry-run`
  # tables are written under this: in a run that upgrades, a closed pipe leaves
  # accepted upgrades undone, which must not end as a success.
  defp writing(fun) do
    fun.()
  rescue
    e in ErlangError ->
      if Map.get(e, :original) in [:terminated, :epipe],
        do: :ok,
        else: reraise(e, __STACKTRACE__)
  end

  defp run(opts) do
    {groups, skipped} = groups(opts)
    dates = release_dates(groups)
    upgradable = Enum.filter(groups, fn {_, rows} -> Enum.any?(rows, & &1.upgradable) end)

    failed =
      cond do
        opts[:dry_run] and groups == [] ->
          IO.puts("All tools are up to date.")
          []

        opts[:dry_run] ->
          writing(fn -> Enum.each(groups, &render(&1, dates)) end)
          []

        upgradable == [] ->
          IO.puts("Nothing to upgrade within the requested versions.")
          []

        true ->
          upgradable |> ask(dates) |> upgrade_all()
      end

    # A directory mise cannot list, such as one whose config is not trusted, is
    # left out rather than stopping the others; mise's own error has already
    # gone to stderr, and the directory is named again last, where it is read.
    if skipped != [] do
      IO.puts(
        :stderr,
        "\nSkipped, as mise could not list them: " <> Enum.map_join(skipped, ", ", &label/1)
      )
    end

    if failed == [] do
      0
    else
      IO.puts(:stderr, "Upgrade failed in: " <> Enum.map_join(failed, ", ", &label/1))
      1
    end
  end

  # Every question comes before any upgrade, so the run needs nobody once the
  # last one is answered. End of input answers no for the rest.
  defp ask(groups, dates) do
    Enum.reduce_while(groups, [], fn {where, _} = group, accepted ->
      render(group, dates)

      case yes?("Upgrade #{label(where)}? [y/N] ") do
        :eof ->
          IO.puts("")
          {:halt, accepted}

        true ->
          {:cont, [where | accepted]}

        false ->
          {:cont, accepted}
      end
    end)
    |> Enum.reverse()
  end

  defp yes?(prompt) do
    case IO.gets(prompt) do
      :eof ->
        :eof

      answer ->
        case answer |> String.trim() |> String.downcase() do
          a when a in ["y", "yes"] -> true
          a when a in ["", "n", "no"] -> false
          _ -> yes?(prompt)
        end
    end
  end

  # One failure leaves the other directories worth upgrading, so the failed
  # ones are returned for reporting once all have run.
  defp upgrade_all([]) do
    IO.puts("Nothing upgraded.")
    []
  end

  defp upgrade_all(accepted), do: Enum.reject(accepted, &upgraded?/1)

  defp upgraded?(where) do
    args =
      if where == :global,
        do: ["upgrade", "--global"],
        else: ["-C", where, "upgrade", "--no-global"]

    IO.puts("\n==> mise #{Enum.join(args, " ")}")
    {_, code} = mise_cmd(args, into: IO.stream())
    code == 0
  end

  # The global config and each directory holding a tracked project config, with
  # the rows its own `mise outdated` reports; a directory with none is dropped.
  defp groups(opts) do
    results =
      [:global | dirs(opts)]
      |> Task.async_stream(&{&1, rows(&1)}, max_concurrency: 8, timeout: :infinity)
      |> Enum.map(fn {:ok, result} -> result end)

    {for({where, {:ok, rows}} <- results, rows != [], do: {where, rows}),
     for({where, :error} <- results, do: where)}
  end

  # Each directory is named by the project root mise loads its config from. The
  # global config's own directory needs no exception: `--no-global` leaves the
  # global config out of a project's view under whatever name it is reached by.
  defp dirs(opts) do
    temp = if opts[:include_tmpdirs], do: [], else: temp_dirs()

    mise!(["config", "ls", "--tracked-configs"])
    |> String.split("\n", trim: true)
    |> Enum.map(&project_dir/1)
    |> Enum.uniq()
    |> Enum.reject(fn dir ->
      scratch?(dir, temp) or (not opts[:include_worktrees] and linked_worktree?(dir))
    end)
    |> Enum.sort()
  end

  # Asked of the view `mise upgrade` acts on there, which holds what the
  # directory inherits and what its lockfile pins. A tool newer only past its
  # request shows up in the `--bump` run alone, and is not upgradable.
  defp rows(where) do
    base =
      if where == :global,
        do: ["-C", "/", "outdated", "--global"],
        else: ["-C", where, "outdated", "--no-global"]

    with {:ok, pinned} <- mise(base ++ ["--json"]),
         {:ok, bumped} <- mise(base ++ ["--bump", "--json"]) do
      {:ok, merge(JSON.decode!(pinned), JSON.decode!(bumped))}
    end
  end

  defp merge(pinned, bumped) do
    (Map.keys(pinned) ++ Map.keys(bumped))
    |> Enum.uniq()
    |> Enum.sort()
    |> Enum.map(fn tool ->
      pin = pinned[tool]
      bump = bumped[tool]

      %{
        tool: tool,
        requested: (pin || bump)["requested"],
        current: (pin || bump)["current"],
        latest: (pin && pin["latest"]) || bump["current"],
        # `bump` names the version to request; it is null when the request
        # already reaches the newest version.
        bump_latest: bump && bump["bump"] && bump["latest"],
        upgradable: pin != nil
      }
    end)
  end

  defp label(:global), do: "global"
  defp label(dir), do: abbreviate(dir)

  # A config file may sit in a directory of its own under the project it
  # configures, as {directory, file name prefix}; mise loads each of these from
  # the project root, which is where `mise upgrade` is run and what is shown.
  @config_dirs [
    {[".config", "mise", "conf.d"], ""},
    {[".mise", "conf.d"], ""},
    {["mise", "conf.d"], ""},
    {[".config", "mise"], "config."},
    {[".config", "mise"], "mise."},
    {[".config"], "mise."},
    {[".mise"], "config."},
    {["mise"], "config."}
  ]

  defp project_dir(path) do
    parts = path |> Path.dirname() |> Path.split()
    name = Path.basename(path)

    Enum.find_value(@config_dirs, Path.join(parts), fn {suffix, prefix} ->
      if List.ends_with?(parts, suffix) and String.starts_with?(name, prefix),
        do: parts |> Enum.drop(-length(suffix)) |> Path.join()
    end)
  end

  # A config under a temporary directory belongs to a trial, an agent's
  # scratchpad among them, and not to anything worth keeping current.
  defp scratch?(dir, temp) do
    Enum.any?(temp, &(dir == &1 or String.starts_with?(dir, &1 <> "/")))
  end

  # Each temporary directory under the name it is given and the one it resolves
  # to, since mise reports a config by the resolved path: `/tmp` is a symlink on
  # macOS.
  defp temp_dirs do
    [System.tmp_dir(), "/tmp"]
    |> Enum.reject(&is_nil/1)
    |> Enum.flat_map(&[Path.expand(&1), resolve(&1)])
    |> Enum.reject(&is_nil/1)
    |> Enum.map(&String.trim_trailing(&1, "/"))
    |> Enum.uniq()
  end

  # A missing `pwd` raises, and is no reason to stop the run.
  defp resolve(path) do
    case Dotfiles.Command.cmd("pwd", ["-P"], cd: path, stderr_to_stdout: true) do
      {out, 0} -> String.trim(out)
      _ -> nil
    end
  rescue
    _ in ErlangError -> nil
  end

  # A linked worktree is an agent's short-lived copy of a checkout, and not a
  # place to keep tools current; git tells one apart by a git dir of its own
  # under the common one, wherever the worktree was placed.
  defp linked_worktree?(dir) do
    case Dotfiles.Command.cmd(
           "git",
           ["-C", dir, "rev-parse", "--path-format=absolute", "--git-dir", "--git-common-dir"],
           stderr_to_stdout: true
         ) do
      {out, 0} ->
        [git_dir, common_dir] = String.split(out, "\n", trim: true)
        git_dir != common_dir

      _ ->
        false
    end
  end

  defp release_dates(groups) do
    tools = for {_, rows} <- groups, row <- rows, uniq: true, do: row.tool

    tools
    |> Task.async_stream(&remote_versions/1, max_concurrency: 8, timeout: :infinity)
    |> Enum.zip(tools)
    |> Map.new(fn {{:ok, versions}, tool} -> {tool, versions} end)
  end

  # `:unavailable` is what could not be asked, and is kept apart from a version
  # the backend lists without a timestamp: both leave the age unknown, but only
  # the first means the report is missing something it could have had. A missing
  # `mise` raises, and is no reason to stop the run.
  defp remote_versions(tool) do
    with {out, 0} <- mise_cmd(["ls-remote", tool, "--json"]),
         {:ok, versions} when is_list(versions) <- JSON.decode(out) do
      Map.new(versions, &{&1["version"], &1["created_at"]})
    else
      _ -> :unavailable
    end
  rescue
    _ in ErlangError -> :unavailable
  end

  defp render({where, rows}, dates) do
    cells =
      Enum.map(rows, fn row ->
        released = &age(Map.get(dates, row.tool, :unavailable), &1)

        [
          row.tool,
          fmt_version(row.requested),
          fmt_version(row.current),
          released.(row.current),
          fmt_version(row.latest),
          released.(row.latest),
          fmt_version(row.bump_latest),
          released.(row.bump_latest)
        ]
      end)

    widths =
      [@headers | cells]
      |> Enum.zip_with(fn col -> col |> Enum.map(&String.length/1) |> Enum.max() end)

    print_row = fn line ->
      line
      |> Enum.zip_with(widths, &String.pad_trailing/2)
      |> Enum.join("  ")
      |> String.trim_trailing()
      |> IO.puts()
    end

    IO.puts("\n" <> label(where))
    print_row.(@headers)
    print_row.(Enum.map(widths, &String.duplicate("-", &1)))
    Enum.each(cells, print_row)
  end

  defp abbreviate(path) do
    home = System.user_home()

    if home && String.starts_with?(path, home <> "/"),
      do: "~" <> String.replace_prefix(path, home, ""),
      else: path
  end

  defp fmt_version(nil), do: "-"
  defp fmt_version(version), do: version

  # A cell with no version to date reads `-` even where the lookup failed.
  defp age(_versions, version) when version in [nil, ""], do: "-"

  defp age(:unavailable, _version), do: "?"

  defp age(versions, version), do: fmt_age(versions[version])

  defp fmt_age(iso) when iso in [nil, ""], do: "-"

  defp fmt_age(iso) do
    case parse_datetime(iso) do
      {:ok, dt} ->
        days = div(DateTime.diff(DateTime.utc_now(), dt), 86_400)
        if days < 1, do: "today", else: "#{days}d ago"

      :error ->
        "?"
    end
  end

  # Accept both offset-aware timestamps (e.g. "...Z") and naive ones
  # (e.g. "2026-06-04T15:34:21"), treating the latter as UTC.
  defp parse_datetime(iso) do
    case DateTime.from_iso8601(iso) do
      {:ok, dt, _} ->
        {:ok, dt}

      _ ->
        case NaiveDateTime.from_iso8601(iso) do
          {:ok, ndt} -> {:ok, DateTime.from_naive!(ndt, "Etc/UTC")}
          _ -> :error
        end
    end
  end

  # mise's stderr is left to the terminal: its warnings would break the JSON
  # read from stdout, and its errors are what the user needs to see.
  defp mise(args) do
    case mise_cmd(args) do
      {out, 0} -> {:ok, out}
      _ -> :error
    end
  end

  defp mise!(args) do
    case mise(args) do
      {:ok, out} -> out
      :error -> die("mise #{Enum.join(args, " ")} failed", 1)
    end
  end

  defp mise_cmd(args, opts \\ []), do: Dotfiles.Command.cmd("mise", args, opts)

  defp die(message, code), do: throw({:die, message, code})
end

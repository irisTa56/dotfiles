#!/usr/bin/env elixir

defmodule MiseOutdatedAge do
  @usage """
  Usage: mise-outdated-age [--paths | --upgrade]

  Reports outdated tools across every mise config tracked on this host, with the
  age of each version involved.

    --paths, -p    also list the config files requesting each tool, not only their number
    --upgrade, -u  list the directories with a tool newer within its request, and run
                   `mise upgrade` in the ones chosen, leaving each request as it is
    --help, -h     print this message

  The upgrade list leaves out linked git worktrees and configs under a temporary
  directory, and upgrades the global config as a group of its own.

  Latest is the newest version matching the request; Bump Latest is the newest
  version overall, reachable only by changing the request. Configs counts the
  config files on disk that make the request, a worktree's copy among them.

  A `-` marks nothing to report: no version installed here (Current), nothing
  newer outside the pin (Bump Latest), or no timestamp published for the version
  (Released). A `?` marks a Released cell whose age could not be determined.
  """

  @headers [
    "Tool",
    "Requested",
    "Current",
    "Released",
    "Latest",
    "Released",
    "Bump Latest",
    "Released",
    "Configs"
  ]

  def main(argv) do
    writing(fn ->
      case argv do
        [] -> run(false)
        [flag] when flag in ["--paths", "-p"] -> run(true)
        [flag] when flag in ["--upgrade", "-u"] -> upgrade()
        [flag] when flag in ["--help", "-h"] -> IO.puts(@usage)
        _ -> die(@usage, 2)
      end
    end)
  end

  # A reader that stops early — `| head`, or quitting the pager — closes the pipe
  # under us, and the next write raises. Left to propagate it exits non-zero and
  # dumps several megabytes of VM state into whatever directory the report was
  # run from, so the report is over rather than failed.
  defp writing(fun) do
    fun.()
  rescue
    e in ErlangError ->
      if e.original in [:terminated, :epipe],
        do: System.halt(0),
        else: reraise(e, __STACKTRACE__)
  end

  defp run(show_paths) do
    requests = collect_requests()

    rows =
      requests
      |> batches()
      |> Enum.flat_map(&outdated/1)
      |> Enum.sort_by(&{&1.tool, &1.requested})

    cond do
      requests == [] -> IO.puts("No tracked mise config requests a tool version.")
      rows == [] -> IO.puts("All tools are up to date.")
      true -> render(rows, release_dates(rows), show_paths)
    end
  end

  defp upgrade do
    groups =
      collect_requests()
      |> batches()
      |> Enum.flat_map(&outdated/1)
      |> by_directory()

    if groups == [] do
      IO.puts("Nothing to upgrade within the requested versions.")
    else
      groups
      |> Enum.with_index(1)
      |> Enum.each(fn {{dir, tools}, n} ->
        IO.puts("#{n}  #{label(dir)}")

        Enum.each(tools, fn t ->
          IO.puts("     #{t.tool}  #{fmt_version(t.current)} -> #{t.latest}")
        end)
      end)

      failed =
        IO.gets("Upgrade which? (numbers, space-separated) ")
        |> choose(length(groups))
        |> Enum.map(&elem(Enum.at(groups, &1 - 1), 0))
        |> Enum.reject(&upgraded?/1)

      if failed != [] do
        die("Upgrade failed in: " <> Enum.map_join(failed, ", ", &label/1), 1)
      end
    end
  end

  # One failure leaves the other directories worth upgrading, so it is reported
  # once they have run rather than stopping them.
  defp upgraded?(dir) do
    args =
      if dir == :global,
        do: ["upgrade", "--global"],
        else: ["-C", dir, "upgrade", "--no-global"]

    IO.puts("\n==> mise #{Enum.join(args, " ")}")
    {_, code} = System.cmd("mise", args, into: IO.stream())
    code == 0
  end

  defp label(:global), do: "global"
  defp label(dir), do: abbreviate(dir)

  # Rows the pinned run reports, that is upgradable without changing the request,
  # as one group per directory to run `mise upgrade` in. The global config is a
  # group of its own whatever name it is reached by, since `--global` is how mise
  # upgrades it apart from the projects that inherit it.
  defp by_directory(rows) do
    global = global_configs()
    temp = temp_dirs()

    for row <- rows, row.latest != row.current, path <- row.paths do
      if file_id(path) in global, do: {:global, row}, else: {project_dir(path), row}
    end
    |> Enum.group_by(&elem(&1, 0), &elem(&1, 1))
    |> Enum.map(fn {dir, tools} -> {dir, Enum.sort_by(tools, & &1.tool)} end)
    |> Enum.reject(fn {dir, _} ->
      dir != :global and (scratch?(dir, temp) or linked_worktree?(dir))
    end)
    |> Enum.sort()
  end

  # A config file may sit in a directory of its own under the project it
  # configures, as {directory, file name prefix}; mise loads each of these from
  # the project root, which is where `mise upgrade` is run and what is shown.
  @config_dirs [
    {[".config", "mise", "conf.d"], ""},
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

  # Run from `/`, mise reads no project config, so what it lists is the global
  # and system config that `mise upgrade --global` upgrades.
  defp global_configs do
    mise!(["-C", "/", "config", "ls", "--json"])
    |> JSON.decode!()
    |> MapSet.new(&file_id(&1["path"]))
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

  defp resolve(path) do
    case System.cmd("pwd", ["-P"], cd: path, stderr_to_stdout: true) do
      {out, 0} -> String.trim(out)
      _ -> nil
    end
  rescue
    _ -> nil
  end

  # An agent's worktree is upgraded through the checkout it was cut from, whose
  # config it copies; git tells one apart by a git dir of its own under the
  # common one, wherever the worktree was placed.
  defp linked_worktree?(dir) do
    case System.cmd(
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

  defp choose(answer, count) when is_binary(answer) do
    answer
    |> String.split()
    |> Enum.flat_map(fn word ->
      case Integer.parse(word) do
        {n, ""} when n in 1..count//1 -> [n]
        _ -> []
      end
    end)
    |> Enum.uniq()
  end

  defp choose(_eof, _count), do: []

  # Every tool version tracked on this host, as {tool, requested, config paths}.
  # Configs mise still tracks after the file is gone are dropped.
  defp collect_requests do
    mise!(["ls", "--all-sources", "--json"])
    |> JSON.decode!()
    |> Enum.flat_map(fn {tool, entries} ->
      for entry <- entries,
          source <- Map.get(entry, "sources", []),
          path = source["path"],
          requested = source["requested_version"],
          File.exists?(path) do
        {{tool, requested}, path}
      end
    end)
    |> Enum.group_by(&elem(&1, 0), &elem(&1, 1))
    |> Enum.map(fn {{tool, requested}, paths} ->
      {tool, requested, paths |> Enum.sort() |> Enum.uniq_by(&file_id/1)}
    end)
    |> Enum.sort()
  end

  # One file reached under two names is one config to edit: this host's global
  # mise config is a symlink into this repository, and mise reports both names
  # as sources, so identity comes from the file rather than from the string.
  defp file_id(path) do
    case File.stat(path) do
      {:ok, stat} -> {stat.major_device, stat.inode}
      _ -> path
    end
  end

  # `mise outdated --json` keys its output by tool name, so one batch may name a
  # tool once; further requests for that tool spill into the following batches.
  defp batches(requests) do
    requests
    |> Enum.reduce([], &place/2)
    |> Enum.map(&Enum.reverse/1)
  end

  defp place(request, batches), do: place(request, batches, [])

  defp place(request, [], done), do: Enum.reverse([[request] | done])

  defp place({tool, _, _} = request, [batch | rest], done) do
    if Enum.any?(batch, fn {t, _, _} -> t == tool end) do
      place(request, rest, [batch | done])
    else
      Enum.reverse(done, [[request | batch] | rest])
    end
  end

  # A tool pinned to a major or exact version can be up to date within its pin
  # yet have a newer version outside it, in which case only the `--bump` run
  # reports it. Both runs are read so neither kind of row is lost.
  defp outdated(batch) do
    specs = Enum.map(batch, fn {tool, requested, _} -> "#{tool}@#{requested}" end)
    pinned = JSON.decode!(mise!(["outdated", "--json" | specs]))
    bumped = JSON.decode!(mise!(["outdated", "--bump", "--json" | specs]))

    Enum.flat_map(batch, fn {tool, requested, paths} ->
      case {pinned[tool], bumped[tool]} do
        {nil, nil} ->
          []

        {pin, bump} ->
          [
            %{
              tool: tool,
              requested: requested,
              current: (pin || bump)["current"],
              # Without `--bump`, `latest` is the newest version matching the pin;
              # a tool missing from that run has nothing newer within its pin.
              latest: (pin && pin["latest"]) || (bump && bump["current"]),
              # `bump` names the version to request; it is null when the pin
              # already reaches the newest version, and `latest` then repeats
              # what the pinned run reported.
              bump_latest: bump && bump["bump"] && bump["latest"],
              paths: paths
            }
          ]
      end
    end)
  end

  defp release_dates(rows) do
    tools = rows |> Enum.map(& &1.tool) |> Enum.uniq()

    tools
    |> Task.async_stream(&remote_versions/1, max_concurrency: 8, timeout: :infinity)
    |> Enum.zip(tools)
    |> Map.new(fn
      {{:ok, versions}, tool} -> {tool, versions}
      {{:exit, _reason}, tool} -> {tool, :unavailable}
    end)
  end

  # `:unavailable` is what could not be asked, and is kept apart from a version
  # the backend lists without a timestamp: both leave the age unknown, but only
  # the first means the report is missing something it could have had.
  defp remote_versions(tool) do
    case System.cmd("mise", ["ls-remote", tool, "--json"]) do
      {out, 0} -> out |> JSON.decode!() |> Map.new(&{&1["version"], &1["created_at"]})
      _ -> :unavailable
    end
  rescue
    _ -> :unavailable
  end

  defp render(rows, dates, show_paths) do
    cells =
      Enum.map(rows, fn row ->
        released = &age(Map.get(dates, row.tool, :unavailable), &1)

        [
          row.tool,
          row.requested,
          fmt_version(row.current),
          released.(row.current),
          fmt_version(row.latest),
          released.(row.latest),
          fmt_version(row.bump_latest),
          released.(row.bump_latest),
          Integer.to_string(length(row.paths))
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

    print_row.(@headers)
    print_row.(Enum.map(widths, &String.duplicate("-", &1)))

    rows
    |> Enum.zip(cells)
    |> Enum.each(fn {row, line} ->
      print_row.(line)
      if show_paths, do: Enum.each(row.paths, &IO.puts("    " <> abbreviate(&1)))
    end)
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

  defp mise!(args) do
    case System.cmd("mise", args) do
      {out, 0} ->
        out

      {out, code} ->
        msg = String.trim(out)
        cmd = Enum.join(["mise" | args], " ")
        die(if(msg == "", do: "#{cmd} failed (exit #{code})", else: msg), code)
    end
  end

  defp die(message, code) do
    IO.puts(:stderr, message)
    System.halt(code)
  end
end

MiseOutdatedAge.main(System.argv())

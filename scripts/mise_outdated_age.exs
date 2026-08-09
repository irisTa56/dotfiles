#!/usr/bin/env elixir

defmodule MiseOutdatedAge do
  @usage """
  Usage: mise-outdated-age [--paths]

  Reports outdated tools across every mise config tracked on this host, with the
  age of each version involved.

    --paths, -p  also list the config files requesting each tool, not only their number
    --help, -h   print this message

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

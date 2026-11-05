#!/usr/bin/env elixir

defmodule MiseMultiUpgrade do
  @usage """
  Usage: mise run mise:multi-upgrade [--dry-run] [--include-worktrees] [--include-tmpdirs]

  Shows, for each directory with a tracked mise config, the tools `mise upgrade`
  there would upgrade, with the age of each version, and asks whether to upgrade
  it. Once every directory is answered, runs `mise upgrade` in the accepted ones,
  keeping each version request as it is. The global config counts as a directory
  of its own, upgraded with `mise upgrade --global`; a project is upgraded with
  `--no-global`, so a global tool is upgraded once.

    --dry-run            show every directory's table, and ask and upgrade nothing
    --include-worktrees  also process linked git worktrees, such as an agent's
    --include-tmpdirs    also process configs under a temporary directory
    --help, -h           print this message

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

  def main(argv) do
    case OptionParser.parse(argv, strict: @switches, aliases: [h: :help]) do
      {opts, [], []} ->
        opts = Keyword.merge(Enum.map(@switches, &{elem(&1, 0), false}), opts)
        writing(fn -> if opts[:help], do: IO.puts(@usage), else: run(opts) end)

      _ ->
        die(@usage, 2)
    end
  end

  # A reader that stops early — `| head`, or quitting the pager — closes the pipe
  # under us, and the next write raises. Left to propagate it exits non-zero and
  # dumps several megabytes of VM state into whatever directory the report was
  # run from, so the report is over rather than failed.
  defp writing(fun) do
    fun.()
  rescue
    e in ErlangError ->
      if Map.get(e, :original) in [:terminated, :epipe],
        do: System.halt(0),
        else: reraise(e, __STACKTRACE__)
  end

  defp run(opts) do
    groups = groups(opts)
    dates = release_dates(groups)
    upgradable = Enum.filter(groups, fn {_, rows} -> Enum.any?(rows, & &1.upgradable) end)

    cond do
      opts[:dry_run] and groups == [] -> IO.puts("All tools are up to date.")
      opts[:dry_run] -> Enum.each(groups, &render(&1, dates))
      upgradable == [] -> IO.puts("Nothing to upgrade within the requested versions.")
      true -> upgradable |> ask(dates) |> upgrade_all()
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

  # One failure leaves the other directories worth upgrading, so it is reported
  # once they have run rather than stopping them.
  defp upgrade_all([]), do: IO.puts("Nothing upgraded.")

  defp upgrade_all(accepted) do
    failed = Enum.reject(accepted, &upgraded?/1)

    if failed != [] do
      die("Upgrade failed in: " <> Enum.map_join(failed, ", ", &label/1), 1)
    end
  end

  defp upgraded?(where) do
    args =
      if where == :global,
        do: ["upgrade", "--global"],
        else: ["-C", where, "upgrade", "--no-global"]

    IO.puts("\n==> mise #{Enum.join(args, " ")}")
    {_, code} = System.cmd("mise", args, into: IO.stream())
    code == 0
  end

  # The global config and each directory holding a tracked project config, with
  # the rows its own `mise outdated` reports; a directory with none is dropped.
  defp groups(opts) do
    [:global | dirs(opts)]
    |> Task.async_stream(&{&1, rows(&1)}, max_concurrency: 8, timeout: :infinity)
    |> Enum.flat_map(fn {:ok, {where, rows}} -> if rows == [], do: [], else: [{where, rows}] end)
  end

  # Each directory is named by the project root mise loads its config from. The
  # global config is left to its own group under whatever name it is reached by,
  # since `--global` is how mise upgrades it without the projects that inherit it.
  defp dirs(opts) do
    global = global_configs()
    temp = if opts[:include_tmpdirs], do: [], else: temp_dirs()

    mise!(["config", "ls", "--tracked-configs"])
    |> String.split("\n", trim: true)
    |> Enum.filter(&File.exists?/1)
    |> Enum.reject(&(file_id(&1) in global))
    |> Enum.map(&project_dir/1)
    |> Enum.uniq()
    |> Enum.reject(&scratch?(&1, temp))
    |> Enum.reject(&(not opts[:include_worktrees] and linked_worktree?(&1)))
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

    pinned = JSON.decode!(mise!(base ++ ["--json"]))
    bumped = JSON.decode!(mise!(base ++ ["--bump", "--json"]))

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

  # A linked worktree is an agent's short-lived copy of a checkout, and not a
  # place to keep tools current; git tells one apart by a git dir of its own
  # under the common one, wherever the worktree was placed.
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

  # An answer is taken whole or not at all: running the part that parsed would
  # leave a mistyped group looking upgraded. An empty answer chooses nothing.
  # One file reached under two names is one config to edit: this host's global
  # mise config is a symlink into this repository, and mise reports both names
  # as sources, so identity comes from the file rather than from the string.
  defp file_id(path) do
    case File.stat(path) do
      {:ok, stat} -> {stat.major_device, stat.inode}
      _ -> path
    end
  end

  defp release_dates(groups) do
    tools = for {_, rows} <- groups, row <- rows, uniq: true, do: row.tool

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

MiseMultiUpgrade.main(System.argv())

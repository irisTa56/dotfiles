defmodule Dotfiles.MiseMultiUpgradeTest do
  use ExUnit.Case
  import ExUnit.CaptureIO

  alias Dotfiles.FakeCommand

  # The machine the code sees: the configs mise tracks, what mise answers in each
  # directory, which directories are git checkouts or linked worktrees, and which
  # paths resolve elsewhere. Directories are asked about from tasks of their own,
  # so it lives in an Agent rather than in the test's process.
  setup do
    {:ok, _} =
      Agent.start_link(
        fn ->
          %{
            tracked: [],
            outdated: %{},
            bump: %{},
            remote: %{},
            worktrees: %{},
            resolved: %{},
            fail: %{}
          }
        end,
        name: __MODULE__
      )

    FakeCommand.install(&answer/3)
    :ok
  end

  defp get(key), do: Agent.get(__MODULE__, &Map.fetch!(&1, key))
  defp set(key, where, value), do: Agent.update(__MODULE__, &put_in(&1, [key, where], value))

  defp answer("mise", ["config", "ls", "--tracked-configs"], _) do
    if get(:fail)[:tracked],
      do: {"", 1},
      else: {Enum.map_join(get(:tracked), &(&1 <> "\n")), 0}
  end

  defp answer("mise", ["-C", "/", "outdated", "--global" | rest], _), do: outdated(:global, rest)
  defp answer("mise", ["-C", dir, "outdated", "--no-global" | rest], _), do: outdated(dir, rest)

  defp answer("mise", ["ls-remote", tool, "--json"], _) do
    case get(:remote)[tool] do
      nil -> {"", 1}
      versions -> {JSON.encode!(versions), 0}
    end
  end

  defp answer("mise", args, _) do
    true = "upgrade" in args
    {"", if(get(:fail)[:upgrade] in args, do: 1, else: 0)}
  end

  # A directory is a checkout of its own unless registered as a linked worktree
  # of one, as {worktree, project}, or as no repository at all.
  defp answer("git", ["-C", dir, "rev-parse", "--path-format=absolute" | _], _) do
    case get(:worktrees)[dir] do
      :none -> {"fatal: not a git repository\n", 128}
      nil -> {"#{dir}/.git\n#{dir}/.git\n", 0}
      project -> {"#{project}/.git/worktrees/#{Path.basename(dir)}\n#{project}/.git\n", 0}
    end
  end

  defp answer("pwd", ["-P"], opts) do
    dir = Keyword.fetch!(opts, :cd)
    {Map.get(get(:resolved), dir, dir) <> "\n", 0}
  end

  defp outdated(where, rest) do
    if where == get(:fail)[:outdated] do
      IO.puts(:stderr, "mise ERROR Config files in #{where}/mise.toml are not trusted.")
      {"", 1}
    else
      answers = if rest == ["--bump", "--json"], do: get(:bump), else: get(:outdated)
      {JSON.encode!(Map.get(answers, where, %{})), 0}
    end
  end

  # Tracks a mise.toml in `dir`, returned.
  defp project(dir) do
    track(Path.join(dir, "mise.toml"))
    dir
  end

  defp track(config),
    do: Agent.update(__MODULE__, &Map.update!(&1, :tracked, fn t -> t ++ [config] end))

  # What `outdated --json` answers in `where` (a project directory, or :global),
  # as %{tool => {current, latest}}: the tools `mise upgrade` there would upgrade.
  defp outdated_in(where, tools) do
    set(
      :outdated,
      where,
      Map.new(tools, fn {tool, {current, latest}} ->
        {tool, %{"current" => current, "latest" => latest, "bump" => nil}}
      end)
    )
  end

  # What `outdated --bump --json` answers in `where`, as
  # %{tool => {current, latest overall, request to bump to}}.
  defp bump_in(where, tools) do
    set(
      :bump,
      where,
      Map.new(tools, fn {tool, {current, latest, bump}} ->
        {tool, %{"current" => current, "latest" => latest, "bump" => bump}}
      end)
    )
  end

  # The exit status, everything written to stdout and then stderr, and the upgrades run.
  defp run(args, answers) do
    {{code, out}, err} =
      with_io(:stderr, fn -> with_io(answers, fn -> Dotfiles.MiseMultiUpgrade.main(args) end) end)

    upgrades = for args <- FakeCommand.calls("mise"), "upgrade" in args, do: Enum.join(args, " ")
    {out <> err, code, upgrades}
  end

  test "asks for each directory, then upgrades the accepted ones after the last answer" do
    a = project("/work/a")
    b = project("/work/b")
    outdated_in(a, %{"jq" => {"1.6", "1.7"}})
    outdated_in(b, %{"yq" => {"4.1", "4.2"}})

    {out, 0, log} = run([], "y\nn\n")

    assert log == ["-C #{a} upgrade --no-global"]
    [asked, ran] = String.split(out, "==> mise", parts: 2)
    assert asked =~ ~r/#{a}.*jq.*1\.6.*1\.7/s
    assert asked =~ ~r/#{b}.*yq.*4\.1.*4\.2/s
    assert ran =~ "-C #{a} upgrade --no-global"
  end

  test "names a directory under the home directory from ~" do
    dir = project(Path.join(System.user_home!(), "work/project"))
    outdated_in(dir, %{"jq" => {"1.6", "1.7"}})

    {out, 0, _} = run([], "n\n")

    assert out =~ "Upgrade ~/work/project? [y/N]"
  end

  test "shows how long ago each version was released" do
    dir = project("/work/project")
    outdated_in(dir, %{"jq" => {"1.6", "1.7"}})
    days_ago = &(DateTime.utc_now() |> DateTime.add(-&1 * 86_400) |> DateTime.to_iso8601())

    set(:remote, "jq", [
      %{"version" => "1.6", "created_at" => days_ago.(30)},
      %{"version" => "1.7", "created_at" => days_ago.(3)}
    ])

    {out, 0, _} = run([], "n\n")

    assert out =~ ~r/^jq .*1\.6\s+30d ago\s+1\.7\s+3d ago/m
  end

  test "reads a release time with no offset as UTC, and one it cannot parse as unknown" do
    dir = project("/work/project")
    outdated_in(dir, %{"jq" => {"1.6", "1.7"}})
    naive = DateTime.utc_now() |> DateTime.add(-5 * 86_400) |> NaiveDateTime.to_iso8601()

    set(:remote, "jq", [
      %{"version" => "1.6", "created_at" => naive},
      %{"version" => "1.7", "created_at" => "last week"}
    ])

    {out, 0, _} = run([], "n\n")

    assert out =~ ~r/^jq .*1\.6\s+5d ago\s+1\.7\s+\?/m
  end

  test "upgrades the global config apart, with --global" do
    dotfiles = project("/work/dotfiles")
    track("/work/dotfiles/.config/mise/config.toml")
    outdated_in(:global, %{"jq" => {"1.6", "1.7"}})
    outdated_in(dotfiles, %{"yq" => {"4.1", "4.2"}})

    {out, 0, log} = run([], "y\ny\n")

    assert out =~ ~r/\A\s*global$/m
    refute out =~ "/work/dotfiles/.config"
    assert log == ["upgrade --global", "-C #{dotfiles} upgrade --no-global"]
  end

  # A project with one tool to upgrade, plus a linked worktree of it that mise
  # also tracks, both with jq outdated in their own view.
  defp with_worktree do
    dir = project("/work/project")
    worktree = project("/work/project/.claude/worktrees/wt")
    set(:worktrees, worktree, dir)
    outdated_in(dir, %{"jq" => {"1.6", "1.7"}})
    outdated_in(worktree, %{"jq" => {"1.6", "1.7"}})
    {dir, worktree}
  end

  test "leaves out a linked worktree" do
    {dir, worktree} = with_worktree()

    {out, 0, log} = run([], "y\ny\n")

    refute out =~ worktree
    assert log == ["-C #{dir} upgrade --no-global"]
  end

  test "keeps a directory git does not know" do
    dir = project("/work/plain")
    set(:worktrees, dir, :none)
    outdated_in(dir, %{"jq" => {"1.6", "1.7"}})

    {_, 0, log} = run([], "y\n")

    assert log == ["-C #{dir} upgrade --no-global"]
  end

  test "takes in a linked worktree with --include-worktrees" do
    {dir, worktree} = with_worktree()

    {_, 0, log} = run(["--include-worktrees"], "y\ny\n")

    assert log == ["-C #{dir} upgrade --no-global", "-C #{worktree} upgrade --no-global"]
  end

  # A project with one tool to upgrade, plus a config under `scratch`, a
  # temporary directory, with the same.
  defp with_scratch(scratch) do
    dir = project("/work/project")
    project(scratch)
    outdated_in(dir, %{"jq" => {"1.6", "1.7"}})
    outdated_in(scratch, %{"jq" => {"1.6", "1.7"}})
    dir
  end

  # TMPDIR has to name a directory that exists, or System.tmp_dir/0 passes it over.
  @tag :tmp_dir
  test "leaves out a config under TMPDIR, by the name it resolves to too", %{tmp_dir: tmp} do
    Dotfiles.TestEnv.put_env([{"TMPDIR", tmp}])
    set(:resolved, tmp, "/resolved/tmp")
    named = with_scratch(Path.join(tmp, "named"))
    project("/resolved/tmp/scratch")
    outdated_in("/resolved/tmp/scratch", %{"jq" => {"1.6", "1.7"}})

    {out, 0, log} = run([], "y\ny\ny\n")

    refute out =~ tmp
    refute out =~ "/resolved/tmp"
    assert log == ["-C #{named} upgrade --no-global"]
  end

  test "leaves out a config under /tmp whatever TMPDIR says, by the name it resolves to" do
    set(:resolved, "/tmp", "/private/tmp")
    dir = with_scratch("/private/tmp/scratch")

    {out, 0, log} = run([], "y\ny\n")

    refute out =~ "/private/tmp"
    assert log == ["-C #{dir} upgrade --no-global"]
  end

  test "takes in a config under a temporary directory with --include-tmpdirs" do
    dir = with_scratch("/tmp/scratch")

    {_, 0, log} = run(["--include-tmpdirs"], "y\ny\n")

    assert Enum.sort(log) ==
             Enum.sort(["-C #{dir} upgrade --no-global", "-C /tmp/scratch upgrade --no-global"])
  end

  test "upgrades every config-directory form from its project root" do
    forms = [
      ".config/mise/conf.d/a.toml",
      ".mise/conf.d/a.toml",
      "mise/conf.d/a.toml",
      ".config/mise/config.toml",
      ".config/mise/mise.toml",
      ".config/mise.toml",
      ".mise/config.toml",
      "mise/config.toml"
    ]

    dirs =
      for {form, i} <- Enum.with_index(forms) do
        dir = "/work/p#{i}"
        track(Path.join(dir, form))
        outdated_in(dir, %{"jq" => {"1.6", "1.7"}})
        dir
      end

    {_, 0, log} = run([], String.duplicate("y\n", length(forms)))

    assert log == Enum.map(dirs, &"-C #{&1} upgrade --no-global")
  end

  test "keeps a project named like a config directory under its own path" do
    dirs =
      for name <- ["mise", ".mise"] do
        dir = project("/work/p#{name}/#{name}")
        outdated_in(dir, %{"jq" => {"1.6", "1.7"}})
        dir
      end

    {_, 0, log} = run([], "y\ny\n")

    assert Enum.sort(log) == Enum.sort(Enum.map(dirs, &"-C #{&1} upgrade --no-global"))
  end

  test "asks once and upgrades once for a directory holding two configs" do
    dir = project("/work/project")
    track(Path.join(dir, "mise.local.toml"))
    outdated_in(dir, %{"jq" => {"1.6", "1.7"}})

    {out, 0, log} = run([], "y\ny\n")

    assert out |> String.split("Upgrade #{dir}?") |> length() == 2
    assert log == ["-C #{dir} upgrade --no-global"]
  end

  test "goes on past a failed upgrade and exits non-zero" do
    a = project("/work/a")
    b = project("/work/b")
    outdated_in(a, %{"jq" => {"1.6", "1.7"}})
    outdated_in(b, %{"jq" => {"1.6", "1.7"}})
    set(:fail, :upgrade, a)

    {out, code, log} = run([], "y\ny\n")

    assert code != 0
    assert out =~ "Upgrade failed in: #{a}"
    assert log == ["-C #{a} upgrade --no-global", "-C #{b} upgrade --no-global"]
  end

  test "goes on past a directory whose outdated fails, and reports it" do
    [a, b, c] = for name <- ~w(a b c), do: project("/work/#{name}")
    for d <- [a, b, c], do: outdated_in(d, %{"jq" => {"1.6", "1.7"}})
    set(:fail, :outdated, b)

    {out, 0, log} = run([], "y\ny\n")

    assert out =~ ~r/#{a}.*jq.*1\.6.*1\.7/s
    assert out =~ ~r/#{c}.*jq.*1\.6.*1\.7/s
    assert log == ["-C #{a} upgrade --no-global", "-C #{c} upgrade --no-global"]
    assert out =~ ~r/#{b}.*not trusted/s
    assert out =~ "Skipped, as mise could not list them: #{b}"
  end

  test "shows every directory at once with --dry-run, asking and upgrading nothing" do
    a = project("/work/a")
    b = project("/work/b")
    outdated_in(a, %{"jq" => {"1.6", "1.7"}})
    bump_in(b, %{"yq" => {"3.4", "4.2", "4"}})

    {out, 0, log} = run(["--dry-run"], "")

    assert log == []
    refute out =~ "Upgrade "
    assert out =~ ~r/#{a}\n.*\n.*\njq /
    assert out =~ ~r/#{b}\n.*\n.*\nyq\s+-\s+3\.4\s+\?\s+3\.4\s+\?\s+4\.2/
  end

  test "asks nothing for a directory newer only past its requests" do
    dir = project("/work/project")
    bump_in(dir, %{"yq" => {"3.4", "4.2", "4"}})

    {out, 0, log} = run([], "y\n")

    refute out =~ dir
    assert out =~ "Nothing to upgrade within the requested versions."
    assert log == []
  end

  test "answers no to an empty line and the rest at end of input, and asks again otherwise" do
    dirs = for name <- ~w(a b c d), do: project("/work/#{name}")
    for d <- dirs, do: outdated_in(d, %{"jq" => {"1.6", "1.7"}})

    [a, b, c, _] = dirs
    {out, 0, log} = run([], "\nmaybe\nY\nyes\n")

    assert log == ["-C #{b} upgrade --no-global", "-C #{c} upgrade --no-global"]
    assert out |> String.split("Upgrade #{b}?") |> length() == 3
    refute out =~ "Upgrade #{a}? [y/N] Upgrade #{a}?"
  end

  test "shows a tool in both answers once, with a bump only where one is named" do
    dir = project("/work/project")
    outdated_in(dir, %{"node" => {"24.1.0", "24.2.0"}, "uv" => {"0.1.0", "0.2.0"}})

    bump_in(dir, %{
      "node" => {"24.1.0", "26.0.0", "26"},
      "uv" => {"0.1.0", "0.2.0", nil}
    })

    {out, 0, _} = run(["--dry-run"], "")

    assert out |> String.split("\nnode ") |> length() == 2
    assert out =~ ~r/^node\s+-\s+24\.1\.0\s+\?\s+24\.2\.0\s+\?\s+26\.0\.0/m
    assert out =~ ~r/^uv\s+-\s+0\.1\.0\s+\?\s+0\.2\.0\s+\?\s+-\s+-$/m
  end

  test "prints its usage for --help and -h, asking and upgrading nothing" do
    dir = project("/work/project")
    outdated_in(dir, %{"jq" => {"1.6", "1.7"}})

    for flag <- ["--help", "-h"] do
      {out, 0, log} = run([flag], "y\n")

      assert out =~ "Usage: mise run mise:multi-upgrade"
      refute out =~ "Upgrade "
      assert log == []
    end
  end

  test "refuses an unknown flag with its usage and exit 2" do
    {out, 2, log} = run(["--bump"], "y\n")

    assert out =~ "Usage: mise run mise:multi-upgrade"
    assert log == []
  end

  # The crashed task logs its error, which is the outcome the test asks for.
  @tag capture_log: true
  test "does not finish normally when the command runner raises for ls-remote" do
    dir = project("/work/project")
    outdated_in(dir, %{"jq" => {"1.6", "1.7"}})

    FakeCommand.install(fn
      "mise", ["ls-remote", _, "--json"], _ -> raise "unexpected call"
      command, args, opts -> answer(command, args, opts)
    end)

    # A raise inside Task.async_stream exits its caller, so main/1 runs in a
    # process of its own, which the test watches instead of joining.
    out =
      capture_io(fn ->
        {pid, ref} =
          spawn_monitor(fn ->
            Process.exit(self(), {:finished, Dotfiles.MiseMultiUpgrade.main(["--dry-run"])})
          end)

        receive do
          {:DOWN, ^ref, :process, ^pid, reason} -> send(self(), {:ended, reason})
        end
      end)

    assert_received {:ended, reason}
    refute match?({:finished, _}, reason), "main/1 finished normally: #{inspect(reason)}\n#{out}"
  end

  test "reads a mise it cannot start, or a list it cannot parse, as release ages unknown" do
    dir = project("/work/project")
    outdated_in(dir, %{"jq" => {"1.6", "1.7"}, "yq" => {"4.1", "4.2"}})

    FakeCommand.install(fn
      "mise", ["ls-remote", "jq", "--json"], _ -> raise ErlangError, original: :enoent
      "mise", ["ls-remote", "yq", "--json"], _ -> {"not json", 0}
      command, args, opts -> answer(command, args, opts)
    end)

    {out, 0, _} = run(["--dry-run"], "")

    assert out =~ ~r/^jq .*1\.6\s+\?\s+1\.7\s+\?/m
    assert out =~ ~r/^yq .*4\.1\s+\?\s+4\.2\s+\?/m
  end

  test "says all tools are up to date when --dry-run has no table to show" do
    project("/work/project")

    {out, 0, _} = run(["--dry-run"], "")

    assert out =~ "All tools are up to date."
  end

  test "says nothing was upgraded when every directory is answered no" do
    dir = project("/work/project")
    outdated_in(dir, %{"jq" => {"1.6", "1.7"}})

    {out, 0, log} = run([], "n\n")

    assert out =~ "Nothing upgraded."
    assert log == []
  end

  test "fails rather than reading as up to date when mise cannot list its configs" do
    outdated_in(:global, %{"jq" => {"1.6", "1.7"}})
    set(:fail, :tracked, true)

    {out, 1, log} = run([], "y\n")

    assert out =~ "mise config ls --tracked-configs failed"
    refute out =~ "up to date"
    assert log == []
  end
end

# A git hook running this suite sets GIT_DIR and the like for its own
# repository, which would turn every git call here, and the code's, toward
# that one.
for {name, _} <- System.get_env(), String.starts_with?(name, "GIT_"), do: System.delete_env(name)

# Answers in mise's place from what a test has set, and logs every upgrade.
# Directories are asked about from tasks of their own, so the state lives in an
# Agent rather than in the test's process.
defmodule FakeMise do
  @behaviour Dotfiles.MiseMultiUpgrade.Mise

  def start do
    Agent.start_link(
      fn -> %{tracked: [], outdated: %{}, bump: %{}, remote: %{}, log: []} end,
      name: __MODULE__
    )
  end

  def get(key), do: Agent.get(__MODULE__, &Map.get(&1, key))
  def put(fun), do: Agent.update(__MODULE__, fun)

  @impl true
  def cmd(["config", "ls", "--tracked-configs"], _) do
    if get(:fail_tracked),
      do: {"", 1},
      else: {Enum.map_join(get(:tracked), &(&1 <> "\n")), 0}
  end

  def cmd(["-C", "/", "outdated", "--global" | rest], _), do: outdated(:global, rest)
  def cmd(["-C", dir, "outdated", "--no-global" | rest], _), do: outdated(dir, rest)

  def cmd(["ls-remote", tool, "--json"], _) do
    case get(:remote)[tool] do
      nil -> {"", 1}
      versions -> {JSON.encode!(versions), 0}
    end
  end

  def cmd(args, _) do
    if "upgrade" in args do
      put(&Map.update!(&1, :log, fn log -> log ++ [Enum.join(args, " ")] end))
      {"", if(get(:fail_upgrade) in args, do: 1, else: 0)}
    else
      raise "unexpected mise call: #{inspect(args)}"
    end
  end

  defp outdated(where, rest) do
    if where == get(:fail_outdated) do
      IO.puts(:stderr, "mise ERROR Config files in #{where}/mise.toml are not trusted.")
      {"", 1}
    else
      answers = if rest == ["--bump", "--json"], do: get(:bump), else: get(:outdated)
      {JSON.encode!(Map.get(answers, where, %{})), 0}
    end
  end
end

defmodule Dotfiles.MiseMultiUpgradeTest do
  use ExUnit.Case
  import ExUnit.CaptureIO

  # Under /var/tmp rather than System.tmp_dir!, which on Linux is /tmp, a
  # directory the code leaves out whatever TMPDIR says. /var/tmp also sits
  # outside the home directory, which the output shortens to ~, and outside
  # any repository a git call there could find.
  setup do
    dir =
      Path.join("/var/tmp", "mise-multi-upgrade-test-#{System.unique_integer([:positive])}")
      |> tap(&File.mkdir_p!/1)
      |> resolve()

    File.mkdir_p!(Path.join(dir, "tmp"))
    tmpdir = System.get_env("TMPDIR")
    System.put_env("TMPDIR", Path.join(dir, "tmp"))
    {:ok, _} = FakeMise.start()
    Application.put_env(:dotfiles, :mise, FakeMise)

    on_exit(fn ->
      if tmpdir, do: System.put_env("TMPDIR", tmpdir), else: System.delete_env("TMPDIR")
      File.rm_rf!(dir)
    end)

    {:ok, dir: dir}
  end

  defp resolve(path) do
    {out, 0} = System.cmd("pwd", ["-P"], cd: path)
    String.trim(out)
  end

  # A git repository at `path` holding a tracked mise.toml, which is returned.
  defp repo(path) do
    File.mkdir_p!(path)
    git(path, ["init", "-q"])
    config = Path.join(path, "mise.toml")
    File.write!(config, "")
    git(path, ["add", "."])
    git(path, ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "init"])
    track(config)
    config
  end

  defp git(cwd, args), do: {_, 0} = System.cmd("git", args, cd: cwd, stderr_to_stdout: true)

  defp track(config),
    do: FakeMise.put(&Map.update!(&1, :tracked, fn t -> t ++ [config] end))

  defp set(key, value), do: FakeMise.put(&Map.put(&1, key, value))
  defp set(key, where, value), do: FakeMise.put(&put_in(&1, [key, where], value))

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

    {out <> err, code, FakeMise.get(:log)}
  end

  test "asks for each directory, then upgrades the accepted ones after the last answer",
       %{dir: dir} do
    a = Path.join(dir, "a")
    b = Path.join(dir, "b")
    repo(a)
    repo(b)
    outdated_in(a, %{"jq" => {"1.6", "1.7"}})
    outdated_in(b, %{"yq" => {"4.1", "4.2"}})

    {out, 0, log} = run([], "y\nn\n")

    assert log == ["-C #{a} upgrade --no-global"]
    [asked, ran] = String.split(out, "==> mise", parts: 2)
    assert asked =~ ~r/#{Regex.escape(a)}.*jq.*1\.6.*1\.7/s
    assert asked =~ ~r/#{Regex.escape(b)}.*yq.*4\.1.*4\.2/s
    assert ran =~ "-C #{a} upgrade --no-global"
  end

  test "shows how long ago each version was released", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(project)
    outdated_in(project, %{"jq" => {"1.6", "1.7"}})
    days_ago = &(DateTime.utc_now() |> DateTime.add(-&1 * 86_400) |> DateTime.to_iso8601())

    set(:remote, "jq", [
      %{"version" => "1.6", "created_at" => days_ago.(30)},
      %{"version" => "1.7", "created_at" => days_ago.(3)}
    ])

    {out, 0, _} = run([], "n\n")

    assert out =~ ~r/^jq .*1\.6\s+30d ago\s+1\.7\s+3d ago/m
  end

  test "upgrades the global config apart, with --global", %{dir: dir} do
    dotfiles = Path.join(dir, "dotfiles")
    repo(dotfiles)
    target = Path.join(dotfiles, ".config/mise/config.toml")
    File.mkdir_p!(Path.dirname(target))
    File.write!(target, "")
    track(target)
    outdated_in(:global, %{"jq" => {"1.6", "1.7"}})
    outdated_in(dotfiles, %{"yq" => {"4.1", "4.2"}})

    {out, 0, log} = run([], "y\ny\n")

    assert out =~ ~r/\A\s*global$/m
    refute out =~ Path.join(dotfiles, ".config")
    assert log == ["upgrade --global", "-C #{dotfiles} upgrade --no-global"]
  end

  # A project with one tool to upgrade, plus a linked worktree of it that
  # mise also tracks, both with jq outdated in their own view.
  defp with_worktree(dir) do
    project = Path.join(dir, "project")
    repo(project)
    worktree = Path.join(project, ".claude/worktrees/wt")
    git(project, ["worktree", "add", "-q", worktree])
    track(Path.join(worktree, "mise.toml"))
    outdated_in(project, %{"jq" => {"1.6", "1.7"}})
    outdated_in(worktree, %{"jq" => {"1.6", "1.7"}})
    {project, worktree}
  end

  test "leaves out a linked worktree", %{dir: dir} do
    {project, worktree} = with_worktree(dir)

    {out, 0, log} = run([], "y\ny\n")

    refute out =~ worktree
    assert log == ["-C #{project} upgrade --no-global"]
  end

  # A project with one tool to upgrade, plus a config under `scratch`, a
  # temporary directory, with the same.
  defp with_scratch(dir, scratch) do
    project = Path.join(dir, "project")
    repo(project)
    File.mkdir_p!(scratch)
    File.write!(Path.join(scratch, "mise.toml"), "")
    track(Path.join(scratch, "mise.toml"))
    outdated_in(project, %{"jq" => {"1.6", "1.7"}})
    outdated_in(scratch, %{"jq" => {"1.6", "1.7"}})
    project
  end

  test "leaves out a config under TMPDIR", %{dir: dir} do
    scratch = Path.join(dir, "tmp/scratch")
    project = with_scratch(dir, scratch)

    {out, 0, log} = run([], "y\ny\n")

    refute out =~ scratch
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "leaves out a config under /tmp whatever TMPDIR says", %{dir: dir} do
    scratch =
      resolve("/tmp")
      |> Path.join("mise-multi-upgrade-test-#{System.unique_integer([:positive])}")

    on_exit(fn -> File.rm_rf!(scratch) end)
    project = with_scratch(dir, scratch)

    {out, 0, log} = run([], "y\ny\n")

    refute out =~ scratch
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "upgrades every config-directory form from its project root", %{dir: dir} do
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

    projects =
      for {form, i} <- Enum.with_index(forms) do
        project = Path.join(dir, "p#{i}")
        config = Path.join(project, form)
        File.mkdir_p!(Path.dirname(config))
        File.write!(config, "")
        track(config)
        outdated_in(project, %{"jq" => {"1.6", "1.7"}})
        project
      end

    {_, 0, log} = run([], String.duplicate("y\n", length(forms)))

    assert log == Enum.map(projects, &"-C #{&1} upgrade --no-global")
  end

  test "goes on past a failed upgrade and exits non-zero", %{dir: dir} do
    a = Path.join(dir, "a")
    b = Path.join(dir, "b")
    repo(a)
    repo(b)
    outdated_in(a, %{"jq" => {"1.6", "1.7"}})
    outdated_in(b, %{"jq" => {"1.6", "1.7"}})
    set(:fail_upgrade, a)

    {out, code, log} = run([], "y\ny\n")

    assert code != 0
    assert out =~ "Upgrade failed in: #{a}"
    assert log == ["-C #{a} upgrade --no-global", "-C #{b} upgrade --no-global"]
  end

  test "goes on past a directory whose outdated fails, and reports it", %{dir: dir} do
    [a, b, c] = for name <- ~w(a b c), do: Path.join(dir, name)

    for d <- [a, b, c] do
      repo(d)
      outdated_in(d, %{"jq" => {"1.6", "1.7"}})
    end

    set(:fail_outdated, b)

    {out, 0, log} = run([], "y\ny\n")

    assert out =~ ~r/#{Regex.escape(a)}.*jq.*1\.6.*1\.7/s
    assert out =~ ~r/#{Regex.escape(c)}.*jq.*1\.6.*1\.7/s
    assert log == ["-C #{a} upgrade --no-global", "-C #{c} upgrade --no-global"]
    assert out =~ ~r/#{Regex.escape(b)}.*not trusted/s
    assert out =~ "Skipped, as mise could not list them: #{b}"
  end

  test "takes in a linked worktree with --include-worktrees", %{dir: dir} do
    {project, worktree} = with_worktree(dir)

    {_, 0, log} = run(["--include-worktrees"], "y\ny\n")

    assert log == ["-C #{project} upgrade --no-global", "-C #{worktree} upgrade --no-global"]
  end

  test "takes in a config under a temporary directory with --include-tmpdirs", %{dir: dir} do
    scratch = Path.join(dir, "tmp/scratch")
    project = with_scratch(dir, scratch)

    {_, 0, log} = run(["--include-tmpdirs"], "y\ny\n")

    assert Enum.sort(log) ==
             Enum.sort(["-C #{project} upgrade --no-global", "-C #{scratch} upgrade --no-global"])
  end

  test "shows every directory at once with --dry-run, asking and upgrading nothing",
       %{dir: dir} do
    a = Path.join(dir, "a")
    b = Path.join(dir, "b")
    repo(a)
    repo(b)
    outdated_in(a, %{"jq" => {"1.6", "1.7"}})
    bump_in(b, %{"yq" => {"3.4", "4.2", "4"}})

    {out, 0, log} = run(["--dry-run"], "")

    assert log == []
    refute out =~ "Upgrade "
    assert out =~ ~r/#{Regex.escape(a)}\n.*\n.*\njq /
    assert out =~ ~r/#{Regex.escape(b)}\n.*\n.*\nyq\s+-\s+3\.4\s+\?\s+3\.4\s+\?\s+4\.2/
  end

  test "asks nothing for a directory newer only past its requests", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(project)
    bump_in(project, %{"yq" => {"3.4", "4.2", "4"}})

    {out, 0, log} = run([], "y\n")

    refute out =~ project
    assert out =~ "Nothing to upgrade within the requested versions."
    assert log == []
  end

  test "answers no to an empty line and the rest at end of input, and asks again otherwise",
       %{dir: dir} do
    dirs = for name <- ~w(a b c d), do: Path.join(dir, name)

    Enum.each(dirs, fn d ->
      repo(d)
      outdated_in(d, %{"jq" => {"1.6", "1.7"}})
    end)

    [a, b, c, _] = dirs
    {out, 0, log} = run([], "\nmaybe\nY\nyes\n")

    assert log == ["-C #{b} upgrade --no-global", "-C #{c} upgrade --no-global"]
    assert out |> String.split("Upgrade #{b}?") |> length() == 3
    refute out =~ "Upgrade #{a}? [y/N] Upgrade #{a}?"
  end

  test "shows a tool in both answers once, with a bump only where one is named", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(project)
    outdated_in(project, %{"node" => {"24.1.0", "24.2.0"}, "uv" => {"0.1.0", "0.2.0"}})

    bump_in(project, %{
      "node" => {"24.1.0", "26.0.0", "26"},
      "uv" => {"0.1.0", "0.2.0", nil}
    })

    {out, 0, _} = run(["--dry-run"], "")

    assert out |> String.split("\nnode ") |> length() == 2
    assert out =~ ~r/^node\s+-\s+24\.1\.0\s+\?\s+24\.2\.0\s+\?\s+26\.0\.0/m
    assert out =~ ~r/^uv\s+-\s+0\.1\.0\s+\?\s+0\.2\.0\s+\?\s+-\s+-$/m
  end

  test "asks once and upgrades once for a directory holding two configs", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(project)
    File.write!(Path.join(project, "mise.local.toml"), "")
    track(Path.join(project, "mise.local.toml"))
    outdated_in(project, %{"jq" => {"1.6", "1.7"}})

    {out, 0, log} = run([], "y\ny\n")

    assert out |> String.split("Upgrade #{project}?") |> length() == 2
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "prints its usage for --help and -h, asking and upgrading nothing", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(project)
    outdated_in(project, %{"jq" => {"1.6", "1.7"}})

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

  test "says all tools are up to date when --dry-run has no table to show", %{dir: dir} do
    repo(Path.join(dir, "project"))

    {out, 0, _} = run(["--dry-run"], "")

    assert out =~ "All tools are up to date."
  end

  test "says nothing was upgraded when every directory is answered no", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(project)
    outdated_in(project, %{"jq" => {"1.6", "1.7"}})

    {out, 0, log} = run([], "n\n")

    assert out =~ "Nothing upgraded."
    assert log == []
  end

  test "keeps a project named like a config directory under its own path", %{dir: dir} do
    projects =
      for name <- ["mise", ".mise"] do
        project = Path.join([dir, "p#{name}", name])
        repo(project)
        outdated_in(project, %{"jq" => {"1.6", "1.7"}})
        project
      end

    {_, 0, log} = run([], "y\ny\n")

    assert Enum.sort(log) == Enum.sort(Enum.map(projects, &"-C #{&1} upgrade --no-global"))
  end

  test "fails rather than reading as up to date when mise cannot list its configs" do
    outdated_in(:global, %{"jq" => {"1.6", "1.7"}})
    set(:fail_tracked, true)

    {out, 1, log} = run([], "y\n")

    assert out =~ "mise config ls --tracked-configs failed"
    refute out =~ "up to date"
    assert log == []
  end
end

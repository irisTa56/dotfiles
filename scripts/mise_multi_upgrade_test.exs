#!/usr/bin/env elixir

ExUnit.start()

defmodule MiseMultiUpgradeTest do
  use ExUnit.Case

  @script Path.expand("mise_multi_upgrade.exs", __DIR__)

  setup do
    dir =
      Path.join(
        System.tmp_dir!(),
        "mise-multi-upgrade-test-#{System.unique_integer([:positive])}"
      )
      |> tap(&File.mkdir_p!/1)
      |> resolve()

    on_exit(fn -> File.rm_rf!(dir) end)

    Enum.each(
      ["bin", "fake/outdated", "fake/bump", "fake/remote", "tmp", "global"],
      &File.mkdir_p!(Path.join(dir, &1))
    )

    global = Path.join(dir, "global/config.toml")
    File.write!(global, "")
    File.write!(Path.join(dir, "fake/tracked.txt"), global <> "\n")

    # The fake mise answers each directory's `outdated` from files of its own, as
    # the real one answers from that directory's view, and logs every upgrade.
    fake_mise(dir, """
    key() { printf %s "$1" | tr / _; }
    case "$*" in
      "-C "*" outdated "*)
        if [ "$2" = "$(cat "#{dir}/fake/outdated_fail" 2>/dev/null)" ]; then
          echo "mise ERROR Config files in $2/mise.toml are not trusted." >&2
          exit 1
        fi ;;
    esac
    case "$*" in
      "config ls --tracked-configs") cat "#{dir}/fake/tracked.txt" ;;
      "-C / outdated --global --json") cat "#{dir}/fake/outdated/global.json" 2>/dev/null || echo '{}' ;;
      "-C / outdated --global --bump --json") cat "#{dir}/fake/bump/global.json" 2>/dev/null || echo '{}' ;;
      "-C "*" outdated --no-global --json") cat "#{dir}/fake/outdated/$(key "$2").json" 2>/dev/null || echo '{}' ;;
      "-C "*" outdated --no-global --bump --json") cat "#{dir}/fake/bump/$(key "$2").json" 2>/dev/null || echo '{}' ;;
      "ls-remote "*" --json") cat "#{dir}/fake/remote/$(key "$2").json" 2>/dev/null || exit 1 ;;
      *upgrade*)
        echo "$*" >> "#{dir}/upgrade.log"
        [ "$2" = "$(cat "#{dir}/fake/fail" 2>/dev/null)" ] && exit 1
        exit 0 ;;
      *) echo "unexpected: $*" >&2; exit 1 ;;
    esac
    """)

    {:ok, dir: dir}
  end

  defp resolve(path) do
    {out, 0} = System.cmd("pwd", ["-P"], cd: path)
    String.trim(out)
  end

  defp fake_mise(dir, body) do
    path = Path.join([dir, "bin", "mise"])
    File.write!(path, "#!/bin/sh\n" <> body)
    File.chmod!(path, 0o755)
  end

  # A git repository at `path` holding a tracked mise.toml, which is returned.
  defp repo(dir, path) do
    File.mkdir_p!(path)
    git(path, ["init", "-q"])
    config = Path.join(path, "mise.toml")
    File.write!(config, "")
    git(path, ["add", "."])
    git(path, ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "init"])
    track(dir, config)
    config
  end

  defp track(dir, config),
    do: File.write!(Path.join(dir, "fake/tracked.txt"), config <> "\n", [:append])

  defp git(cwd, args) do
    {_, 0} = System.cmd("git", args, cd: cwd, env: no_git_env(), stderr_to_stdout: true)
  end

  # A git hook running this suite sets GIT_DIR and the like for its own
  # repository, which would turn every git call here toward that one.
  defp no_git_env do
    for {name, _} <- System.get_env(), String.starts_with?(name, "GIT_"), do: {name, nil}
  end

  defp key(:global), do: "global"
  defp key(project), do: String.replace(project, "/", "_")

  # What `outdated --json` answers in `where` (a project directory, or :global),
  # as %{tool => {current, latest}}: the tools `mise upgrade` there would upgrade.
  defp outdated_in(dir, where, tools) do
    answer =
      Map.new(tools, fn {tool, {current, latest}} ->
        {tool, %{"current" => current, "latest" => latest, "bump" => nil}}
      end)

    File.write!(Path.join(dir, "fake/outdated/#{key(where)}.json"), JSON.encode!(answer))
  end

  # What `outdated --bump --json` answers in `where`, as
  # %{tool => {current, latest overall, request to bump to}}.
  defp bump_in(dir, where, tools) do
    answer =
      Map.new(tools, fn {tool, {current, latest, bump}} ->
        {tool, %{"current" => current, "latest" => latest, "bump" => bump}}
      end)

    File.write!(Path.join(dir, "fake/bump/#{key(where)}.json"), JSON.encode!(answer))
  end

  defp run(dir, args, answers) do
    env = [
      {"PATH", Path.join(dir, "bin") <> ":" <> System.get_env("PATH")},
      {"TMPDIR", Path.join(dir, "tmp")}
      | no_git_env()
    ]

    {out, code} =
      System.cmd("sh", ["-c", ~s(printf '%s' "$ANSWERS" | elixir "$SCRIPT" "$@"), "sh" | args],
        env: [{"ANSWERS", answers}, {"SCRIPT", @script} | env],
        stderr_to_stdout: true
      )

    log =
      case File.read(Path.join(dir, "upgrade.log")) do
        {:ok, text} -> String.split(text, "\n", trim: true)
        _ -> []
      end

    {out, code, log}
  end

  test "asks for each directory, then upgrades the accepted ones after the last answer",
       %{dir: dir} do
    a = Path.join(dir, "a")
    b = Path.join(dir, "b")
    repo(dir, a)
    repo(dir, b)
    outdated_in(dir, a, %{"jq" => {"1.6", "1.7"}})
    outdated_in(dir, b, %{"yq" => {"4.1", "4.2"}})

    {out, 0, log} = run(dir, [], "y\nn\n")

    assert log == ["-C #{a} upgrade --no-global"]
    [asked, ran] = String.split(out, "==> mise", parts: 2)
    assert asked =~ ~r/#{Regex.escape(a)}.*jq.*1\.6.*1\.7/s
    assert asked =~ ~r/#{Regex.escape(b)}.*yq.*4\.1.*4\.2/s
    assert ran =~ "-C #{a} upgrade --no-global"
  end

  test "shows how long ago each version was released", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(dir, project)
    outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}})
    days_ago = &(DateTime.utc_now() |> DateTime.add(-&1 * 86_400) |> DateTime.to_iso8601())

    File.write!(
      Path.join(dir, "fake/remote/jq.json"),
      JSON.encode!([
        %{"version" => "1.6", "created_at" => days_ago.(30)},
        %{"version" => "1.7", "created_at" => days_ago.(3)}
      ])
    )

    {out, 0, _} = run(dir, [], "n\n")

    assert out =~ ~r/^jq .*1\.6\s+30d ago\s+1\.7\s+3d ago/m
  end

  test "upgrades the global config apart, with --global", %{dir: dir} do
    dotfiles = Path.join(dir, "dotfiles")
    repo(dir, dotfiles)
    target = Path.join(dotfiles, ".config/mise/config.toml")
    File.mkdir_p!(Path.dirname(target))
    File.write!(target, "")
    track(dir, target)
    global = Path.join(dir, "global/config.toml")
    File.rm!(global)
    File.ln_s!(target, global)
    outdated_in(dir, :global, %{"jq" => {"1.6", "1.7"}})
    outdated_in(dir, dotfiles, %{"yq" => {"4.1", "4.2"}})

    {out, 0, log} = run(dir, [], "y\ny\n")

    assert out =~ ~r/\A\s*global$/m
    refute out =~ Path.join(dotfiles, ".config")
    assert log == ["upgrade --global", "-C #{dotfiles} upgrade --no-global"]
  end

  # A project with one tool to upgrade, plus a linked worktree of it that
  # mise also tracks, both with jq outdated in their own view.
  defp with_worktree(dir) do
    project = Path.join(dir, "project")
    repo(dir, project)
    worktree = Path.join(project, ".claude/worktrees/wt")
    git(project, ["worktree", "add", "-q", worktree])
    track(dir, Path.join(worktree, "mise.toml"))
    outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}})
    outdated_in(dir, worktree, %{"jq" => {"1.6", "1.7"}})
    {project, worktree}
  end

  test "leaves out a linked worktree", %{dir: dir} do
    {project, worktree} = with_worktree(dir)

    {out, 0, log} = run(dir, [], "y\ny\n")

    refute out =~ worktree
    assert log == ["-C #{project} upgrade --no-global"]
  end

  # A project with one tool to upgrade, plus a config under `scratch`, a
  # temporary directory, with the same.
  defp with_scratch(dir, scratch) do
    project = Path.join(dir, "project")
    repo(dir, project)
    File.mkdir_p!(scratch)
    File.write!(Path.join(scratch, "mise.toml"), "")
    track(dir, Path.join(scratch, "mise.toml"))
    outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}})
    outdated_in(dir, scratch, %{"jq" => {"1.6", "1.7"}})
    project
  end

  test "leaves out a config under TMPDIR", %{dir: dir} do
    scratch = Path.join(dir, "tmp/scratch")
    project = with_scratch(dir, scratch)

    {out, 0, log} = run(dir, [], "y\ny\n")

    refute out =~ scratch
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "leaves out a config under /tmp whatever TMPDIR says", %{dir: dir} do
    scratch =
      resolve("/tmp")
      |> Path.join("mise-multi-upgrade-test-#{System.unique_integer([:positive])}")

    on_exit(fn -> File.rm_rf!(scratch) end)
    project = with_scratch(dir, scratch)

    {out, 0, log} = run(dir, [], "y\ny\n")

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
        track(dir, config)
        outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}})
        project
      end

    {_, 0, log} = run(dir, [], String.duplicate("y\n", length(forms)))

    assert log == Enum.map(projects, &"-C #{&1} upgrade --no-global")
  end

  test "goes on past a failed upgrade and exits non-zero", %{dir: dir} do
    a = Path.join(dir, "a")
    b = Path.join(dir, "b")
    repo(dir, a)
    repo(dir, b)
    outdated_in(dir, a, %{"jq" => {"1.6", "1.7"}})
    outdated_in(dir, b, %{"jq" => {"1.6", "1.7"}})
    File.write!(Path.join(dir, "fake/fail"), a)

    {out, code, log} = run(dir, [], "y\ny\n")

    assert code != 0
    assert out =~ "Upgrade failed in: #{a}"
    assert log == ["-C #{a} upgrade --no-global", "-C #{b} upgrade --no-global"]
  end

  test "goes on past a directory whose outdated fails, and reports it", %{dir: dir} do
    [a, b, c] = for name <- ~w(a b c), do: Path.join(dir, name)

    for d <- [a, b, c] do
      repo(dir, d)
      outdated_in(dir, d, %{"jq" => {"1.6", "1.7"}})
    end

    File.write!(Path.join(dir, "fake/outdated_fail"), b)

    {out, 0, log} = run(dir, [], "y\ny\n")

    assert out =~ ~r/#{Regex.escape(a)}.*jq.*1\.6.*1\.7/s
    assert out =~ ~r/#{Regex.escape(c)}.*jq.*1\.6.*1\.7/s
    assert log == ["-C #{a} upgrade --no-global", "-C #{c} upgrade --no-global"]
    assert out =~ ~r/#{Regex.escape(b)}.*not trusted/s
    assert out =~ "Skipped, as mise could not list them: #{b}"
  end

  test "takes in a linked worktree with --include-worktrees", %{dir: dir} do
    {project, worktree} = with_worktree(dir)

    {_, 0, log} = run(dir, ["--include-worktrees"], "y\ny\n")

    assert log == ["-C #{project} upgrade --no-global", "-C #{worktree} upgrade --no-global"]
  end

  test "takes in a config under a temporary directory with --include-tmpdirs", %{dir: dir} do
    scratch = Path.join(dir, "tmp/scratch")
    project = with_scratch(dir, scratch)

    {_, 0, log} = run(dir, ["--include-tmpdirs"], "y\ny\n")

    assert Enum.sort(log) ==
             Enum.sort(["-C #{project} upgrade --no-global", "-C #{scratch} upgrade --no-global"])
  end

  test "shows every directory at once with --dry-run, asking and upgrading nothing",
       %{dir: dir} do
    a = Path.join(dir, "a")
    b = Path.join(dir, "b")
    repo(dir, a)
    repo(dir, b)
    outdated_in(dir, a, %{"jq" => {"1.6", "1.7"}})
    bump_in(dir, b, %{"yq" => {"3.4", "4.2", "4"}})

    {out, 0, log} = run(dir, ["--dry-run"], "")

    assert log == []
    refute out =~ "Upgrade "
    assert out =~ ~r/#{Regex.escape(a)}\n.*\n.*\njq /
    assert out =~ ~r/#{Regex.escape(b)}\n.*\n.*\nyq\s+-\s+3\.4\s+\?\s+3\.4\s+\?\s+4\.2/
  end

  test "asks nothing for a directory newer only past its requests", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(dir, project)
    bump_in(dir, project, %{"yq" => {"3.4", "4.2", "4"}})

    {out, 0, log} = run(dir, [], "y\n")

    refute out =~ project
    assert out =~ "Nothing to upgrade within the requested versions."
    assert log == []
  end

  test "answers no to an empty line and the rest at end of input, and asks again otherwise",
       %{dir: dir} do
    dirs = for name <- ~w(a b c d), do: Path.join(dir, name)

    Enum.each(dirs, fn d ->
      repo(dir, d)
      outdated_in(dir, d, %{"jq" => {"1.6", "1.7"}})
    end)

    [a, b, c, _] = dirs
    {out, 0, log} = run(dir, [], "\nmaybe\nY\nyes\n")

    assert log == ["-C #{b} upgrade --no-global", "-C #{c} upgrade --no-global"]
    assert out |> String.split("Upgrade #{b}?") |> length() == 3
    refute out =~ "Upgrade #{a}? [y/N] Upgrade #{a}?"
  end

  test "shows a tool in both answers once, with a bump only where one is named", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(dir, project)
    outdated_in(dir, project, %{"node" => {"24.1.0", "24.2.0"}, "uv" => {"0.1.0", "0.2.0"}})

    bump_in(dir, project, %{
      "node" => {"24.1.0", "26.0.0", "26"},
      "uv" => {"0.1.0", "0.2.0", nil}
    })

    {out, 0, _} = run(dir, ["--dry-run"], "")

    assert out |> String.split("\nnode ") |> length() == 2
    assert out =~ ~r/^node\s+-\s+24\.1\.0\s+\?\s+24\.2\.0\s+\?\s+26\.0\.0/m
    assert out =~ ~r/^uv\s+-\s+0\.1\.0\s+\?\s+0\.2\.0\s+\?\s+-\s+-$/m
  end

  test "asks once and upgrades once for a directory holding two configs", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(dir, project)
    File.write!(Path.join(project, "mise.local.toml"), "")
    track(dir, Path.join(project, "mise.local.toml"))
    outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}})

    {out, 0, log} = run(dir, [], "y\ny\n")

    assert out |> String.split("Upgrade #{project}?") |> length() == 2
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "ends quietly with exit 0 when the reader closes the pipe early", %{dir: dir} do
    for i <- 1..50 do
      project = Path.join(dir, "p#{i}")
      repo(dir, project)
      outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}})
    end

    tmp = Path.join(dir, "tmp")

    env = [
      {"PATH", Path.join(dir, "bin") <> ":" <> System.get_env("PATH")},
      {"TMPDIR", tmp},
      {"SCRIPT", @script}
      | no_git_env()
    ]

    # The script's own exit status and stderr, not those of `head`.
    script =
      ~s({ elixir "$SCRIPT" --dry-run 2>"$TMPDIR/err"; echo $? >"$TMPDIR/code"; } | head -1)

    {_, 0} = System.cmd("sh", ["-c", script], env: env, cd: tmp)

    assert File.read!(Path.join(tmp, "code")) == "0\n"
    refute File.read!(Path.join(tmp, "err")) =~ "** ("
    refute File.exists?(Path.join(tmp, "erl_crash.dump"))
  end

  test "prints its usage for --help and -h, asking and upgrading nothing", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(dir, project)
    outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}})

    for flag <- ["--help", "-h"] do
      {out, 0, log} = run(dir, [flag], "y\n")

      assert out =~ "Usage: mise run mise:multi-upgrade"
      refute out =~ "Upgrade "
      assert log == []
    end
  end

  test "refuses an unknown flag with its usage and exit 2", %{dir: dir} do
    {out, 2, log} = run(dir, ["--bump"], "y\n")

    assert out =~ "Usage: mise run mise:multi-upgrade"
    assert log == []
  end

  test "says all tools are up to date when --dry-run has no table to show", %{dir: dir} do
    repo(dir, Path.join(dir, "project"))

    {out, 0, _} = run(dir, ["--dry-run"], "")

    assert out =~ "All tools are up to date."
  end

  test "says nothing was upgraded when every directory is answered no", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(dir, project)
    outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}})

    {out, 0, log} = run(dir, [], "n\n")

    assert out =~ "Nothing upgraded."
    assert log == []
  end

  test "keeps a project named like a config directory under its own path", %{dir: dir} do
    projects =
      for name <- ["mise", ".mise"] do
        project = Path.join([dir, "p#{name}", name])
        repo(dir, project)
        outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}})
        project
      end

    {_, 0, log} = run(dir, [], "y\ny\n")

    assert Enum.sort(log) == Enum.sort(Enum.map(projects, &"-C #{&1} upgrade --no-global"))
  end
end

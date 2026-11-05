#!/usr/bin/env elixir

ExUnit.start()

defmodule MiseOutdatedAgeTest do
  use ExUnit.Case

  @script Path.expand("mise_outdated_age.exs", __DIR__)

  setup do
    dir =
      Path.join(System.tmp_dir!(), "mise-outdated-age-test-#{System.unique_integer([:positive])}")
      |> tap(&File.mkdir_p!/1)
      |> resolve()

    on_exit(fn -> File.rm_rf!(dir) end)
    Enum.each(["bin", "fake/outdated", "tmp", "global"], &File.mkdir_p!(Path.join(dir, &1)))
    File.write!(Path.join(dir, "global/config.toml"), "")
    File.write!(Path.join(dir, "fake/tracked.txt"), Path.join(dir, "global/config.toml") <> "\n")

    # The fake mise answers from files under fake/ and logs every upgrade it is asked for.
    # The per-directory and global `outdated` answers come from files of their own
    # (fake/outdated/), apart from the host-wide ones, as the real mise's do: they
    # list what `mise upgrade` there would upgrade, which the host-wide `ls` and
    # `outdated <tool>@<request>` answers need not agree with.
    fake_mise(dir, """
    case "$*" in
      "-C / config ls --json") cat "#{dir}/fake/global.json" ;;
      "config ls --tracked-configs") cat "#{dir}/fake/tracked.txt" ;;
      "ls --all-sources --json") cat "#{dir}/fake/ls.json" ;;
      "outdated --global --json"|"-C / outdated --global --json")
        cat "#{dir}/fake/outdated/global.json" 2>/dev/null || echo '{}' ;;
      "-C "*" outdated --no-global --json")
        cat "#{dir}/fake/outdated/$(printf %s "$2" | tr / _).json" 2>/dev/null || echo '{}' ;;
      "outdated --bump --json"*) cat "#{dir}/fake/bumped.json" ;;
      "outdated --json"*) cat "#{dir}/fake/pinned.json" ;;
      *upgrade*)
        echo "$*" >> "#{dir}/upgrade.log"
        [ "$2" = "$(cat "#{dir}/fake/fail" 2>/dev/null)" ] && exit 1
        exit 0 ;;
      *) echo "unexpected: $*" >&2; exit 1 ;;
    esac
    """)

    File.write!(
      Path.join(dir, "fake/global.json"),
      JSON.encode!([%{"path" => Path.join(dir, "global/config.toml")}])
    )

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

  # A git repository at `path` holding a mise.toml, which is returned.
  defp repo(path) do
    File.mkdir_p!(path)
    git(path, ["init", "-q"])
    config = Path.join(path, "mise.toml")
    File.write!(config, "")
    git(path, ["add", "."])
    git(path, ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "init"])
    config
  end

  defp git(cwd, args) do
    {_, 0} = System.cmd("git", args, cd: cwd, env: no_git_env(), stderr_to_stdout: true)
  end

  # A git hook running this suite sets GIT_DIR and the like for its own
  # repository, which would turn every git call here toward that one.
  defp no_git_env do
    for {name, _} <- System.get_env(), String.starts_with?(name, "GIT_"), do: {name, nil}
  end

  # Tools as {tool, requested, current, latest within the request, config paths}.
  defp tools(dir, tools) do
    ls =
      Map.new(tools, fn {tool, requested, _, _, paths} ->
        sources = Enum.map(paths, &%{"path" => &1, "requested_version" => requested})
        {tool, [%{"sources" => sources}]}
      end)

    pinned =
      for {tool, _, current, latest, _} <- tools, latest, into: %{} do
        {tool, %{"current" => current, "latest" => latest}}
      end

    # Where every tool is current and the host-wide answers hold, a directory's own
    # answer is what its configs request; a test sets one apart with `outdated_in/3`.
    configs = tools |> Enum.flat_map(&elem(&1, 4)) |> Enum.uniq()
    File.write!(Path.join(dir, "fake/tracked.txt"), Enum.join(configs, "\n") <> "\n")

    for {tool, _, current, latest, paths} <- tools,
        latest,
        path <- paths,
        Path.basename(path) in ["mise.toml", "mise.local.toml"] do
      project = Path.dirname(path)
      file = outdated_file(dir, project)
      known = if File.exists?(file), do: JSON.decode!(File.read!(file)), else: %{}

      File.write!(
        file,
        JSON.encode!(Map.put(known, tool, %{"current" => current, "latest" => latest}))
      )
    end

    File.write!(Path.join(dir, "fake/ls.json"), JSON.encode!(ls))
    File.write!(Path.join(dir, "fake/pinned.json"), JSON.encode!(pinned))
    File.write!(Path.join(dir, "fake/bumped.json"), "{}")
  end

  defp outdated_file(dir, project),
    do: Path.join(dir, "fake/outdated/#{String.replace(project, "/", "_")}.json")

  # What `mise -C project outdated --no-global --json` answers, as
  # %{tool => {current, latest}}, in place of what `tools/2` derived; the
  # project's `mise.toml` is tracked unless a config under it already is.
  defp outdated_in(dir, project, outdated) do
    answer =
      Map.new(outdated, fn {tool, {current, latest}} ->
        {tool, %{"current" => current, "latest" => latest}}
      end)

    File.write!(outdated_file(dir, project), JSON.encode!(answer))
    tracked = Path.join(dir, "fake/tracked.txt")

    unless tracked |> File.read!() |> String.contains?(project <> "/") do
      File.write!(tracked, Path.join(project, "mise.toml") <> "\n", [:append])
    end
  end

  defp upgrade(dir, answer) do
    env = [
      {"PATH", Path.join(dir, "bin") <> ":" <> System.get_env("PATH")},
      {"TMPDIR", Path.join(dir, "tmp")}
      | no_git_env()
    ]

    {out, code} =
      System.cmd("sh", ["-c", ~s(printf '%s\n' "$ANSWER" | elixir "$SCRIPT" --upgrade)],
        env: [{"ANSWER", answer}, {"SCRIPT", @script} | env],
        stderr_to_stdout: true
      )

    log =
      case File.read(Path.join(dir, "upgrade.log")) do
        {:ok, text} -> String.split(text, "\n", trim: true)
        _ -> []
      end

    {out, code, log}
  end

  test "upgrades the chosen project directory without its global tools", %{dir: dir} do
    project = Path.join(dir, "project")
    tools(dir, [{"jq", "1", "1.6", "1.7", [repo(project)]}])

    {out, 0, log} = upgrade(dir, "1")

    assert out =~ project
    assert out =~ "jq"
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "leaves out a tool newer only outside its request", %{dir: dir} do
    tools(dir, [{"jq", "1.6", "1.6", nil, [repo(Path.join(dir, "project"))]}])

    File.write!(
      Path.join(dir, "fake/bumped.json"),
      JSON.encode!(%{"jq" => %{"current" => "1.6", "latest" => "1.7", "bump" => "1.7"}})
    )

    {out, 0, log} = upgrade(dir, "1")

    assert out =~ "Nothing to upgrade"
    assert log == []
  end

  test "leaves out a linked worktree's copy of a config", %{dir: dir} do
    project = Path.join(dir, "project")
    config = repo(project)
    worktree = Path.join(project, ".claude/worktrees/wt")
    git(project, ["worktree", "add", "-q", worktree])
    tools(dir, [{"jq", "1", "1.6", "1.7", [config, Path.join(worktree, "mise.toml")]}])

    {out, 0, log} = upgrade(dir, "1")

    refute out =~ worktree
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "leaves out a config under the temporary directory", %{dir: dir} do
    project = Path.join(dir, "project")
    scratch = Path.join(dir, "tmp/scratch")
    File.mkdir_p!(scratch)
    File.write!(Path.join(scratch, "mise.toml"), "")
    tools(dir, [{"jq", "1", "1.6", "1.7", [repo(project), Path.join(scratch, "mise.toml")]}])

    {out, 0, log} = upgrade(dir, "1")

    refute out =~ scratch
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "upgrades the global config apart, under each name it is reached by", %{dir: dir} do
    dotfiles = Path.join(dir, "dotfiles")
    config = repo(dotfiles)
    target = Path.join(dotfiles, ".config/mise/config.toml")
    File.mkdir_p!(Path.dirname(target))
    File.write!(target, "")
    global = Path.join(dir, "global/config.toml")
    File.rm!(global)
    File.ln_s!(target, global)

    tools(dir, [
      {"jq", "1", "1.6", "1.7", [global, target]},
      {"yq", "4", "4.1", "4.2", [config]}
    ])

    File.write!(
      Path.join(dir, "fake/outdated/global.json"),
      JSON.encode!(%{"jq" => %{"current" => "1.6", "latest" => "1.7"}})
    )

    {out, 0, log} = upgrade(dir, "1 2")

    assert out =~ ~r/^1  global$/m
    refute out =~ target
    assert log == ["upgrade --global", "-C #{dotfiles} upgrade --no-global"]
  end

  test "names a config in a config directory by the project it belongs to", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(project)
    nested = Path.join(project, ".mise/config.toml")
    File.mkdir_p!(Path.dirname(nested))
    File.write!(nested, "")
    tools(dir, [{"jq", "1", "1.6", "1.7", [nested]}])
    outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}})

    {out, 0, log} = upgrade(dir, "1")

    assert out =~ ~r/^1  #{Regex.escape(project)}$/m
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "says so and exits non-zero on a word that is not an offered number", %{dir: dir} do
    project = Path.join(dir, "project")
    tools(dir, [{"jq", "1", "1.6", "1.8", [repo(project)]}])

    for answer <- ["1,2", "all", "7"] do
      File.rm(Path.join(dir, "upgrade.log"))

      {out, code, log} = upgrade(dir, answer)
      reply = out |> String.split("Upgrade which?") |> List.last()

      assert code != 0, "answer #{inspect(answer)} exited 0"
      assert reply =~ answer, "answer #{inspect(answer)} was not named in: #{inspect(reply)}"
      assert log == [], "answer #{inspect(answer)} ran #{inspect(log)}"
    end

    File.rm(Path.join(dir, "upgrade.log"))
    {out, code, _log} = upgrade(dir, "1 x")

    assert code != 0
    assert out |> String.split("Upgrade which?") |> List.last() =~ "x"
  end

  test "runs nothing and exits 0 on an empty answer", %{dir: dir} do
    tools(dir, [{"jq", "1", "1.6", "1.7", [repo(Path.join(dir, "project"))]}])

    {_out, 0, log} = upgrade(dir, "")

    assert log == []
  end

  test "upgrades a directory once when its number is given twice", %{dir: dir} do
    project = Path.join(dir, "project")
    tools(dir, [{"jq", "1", "1.6", "1.7", [repo(project)]}])

    {_out, 0, log} = upgrade(dir, "1 1")

    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "lists a tool once under a directory holding two configs that request it", %{dir: dir} do
    project = Path.join(dir, "project")
    config = repo(project)
    local = Path.join(project, "mise.local.toml")
    File.write!(local, "")
    tools(dir, [{"jq", "1", "1.6", "1.7", [config, local]}])

    {out, 0, _log} = upgrade(dir, "")

    assert length(Regex.scan(~r/^\s+jq  /m, out)) == 1
  end

  test "goes on past a failed upgrade and exits non-zero", %{dir: dir} do
    a = Path.join(dir, "a")
    b = Path.join(dir, "b")
    tools(dir, [{"jq", "1", "1.6", "1.7", [repo(a), repo(b)]}])
    File.write!(Path.join(dir, "fake/fail"), a)

    {out, code, log} = upgrade(dir, "1 2")

    assert code != 0
    assert out =~ a
    assert log == ["-C #{a} upgrade --no-global", "-C #{b} upgrade --no-global"]
  end

  test "leaves out a config under /tmp whatever TMPDIR says", %{dir: dir} do
    project = Path.join(dir, "project")

    scratch =
      resolve("/tmp") |> Path.join("mise-outdated-age-test-#{System.unique_integer([:positive])}")

    File.mkdir_p!(scratch)
    on_exit(fn -> File.rm_rf!(scratch) end)
    File.write!(Path.join(scratch, "mise.toml"), "")
    tools(dir, [{"jq", "1", "1.6", "1.7", [repo(project), Path.join(scratch, "mise.toml")]}])

    {out, 0, log} = upgrade(dir, "1")

    refute out =~ scratch
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "upgrades every config-directory form from its project root", %{dir: dir} do
    forms = [
      ".config/mise/conf.d/a.toml",
      ".config/mise/config.toml",
      ".config/mise/mise.toml",
      ".config/mise.toml",
      ".mise/config.toml",
      "mise/config.toml"
    ]

    projects =
      for {form, i} <- Enum.with_index(forms) do
        project = Path.join(dir, "p#{i}")
        repo(project)
        config = Path.join(project, form)
        File.mkdir_p!(Path.dirname(config))
        File.write!(config, "")
        {project, config}
      end

    tools(dir, [{"jq", "1", "1.6", "1.7", Enum.map(projects, &elem(&1, 1))}])

    Enum.each(projects, fn {project, _} ->
      outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}})
    end)

    {_, 0, log} = upgrade(dir, Enum.map_join(1..length(forms), " ", &to_string/1))

    assert log == Enum.map(projects, fn {project, _} -> "-C #{project} upgrade --no-global" end)
  end

  test "offers a directory whose own outdated reports a tool that ls omits", %{dir: dir} do
    project = Path.join(dir, "project")
    repo(project)
    tools(dir, [])
    outdated_in(dir, project, %{"communique" => {"1.0", "1.1"}})

    {out, 0, log} = upgrade(dir, "1")

    assert out =~ project
    assert out =~ ~r/^\s+communique  1\.0 -> 1\.1$/m
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "leaves out a tool the host-wide run reports without a current version but the directory does not",
       %{dir: dir} do
    project = Path.join(dir, "project")
    tools(dir, [{"jq", "1", nil, "1.7", [repo(project)]}])
    outdated_in(dir, project, %{})

    {out, 0, log} = upgrade(dir, "1")

    refute out =~ "jq"
    assert out =~ "Nothing to upgrade"
    assert log == []
  end

  test "lists a tool the directory's own outdated reports though none of its configs request it",
       %{dir: dir} do
    project = Path.join(dir, "project")
    tools(dir, [{"jq", "1", "1.6", "1.7", [repo(project)]}])
    outdated_in(dir, project, %{"jq" => {"1.6", "1.7"}, "yq" => {"4.1", "4.2"}})

    {out, 0, _log} = upgrade(dir, "")

    assert out =~ ~r/^\s+jq  1\.6 -> 1\.7$/m
    assert out =~ ~r/^\s+yq  4\.1 -> 4\.2$/m
  end
end

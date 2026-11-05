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
    Enum.each(["bin", "fake", "tmp", "global"], &File.mkdir_p!(Path.join(dir, &1)))
    File.write!(Path.join(dir, "global/config.toml"), "")

    # The fake mise answers from files under fake/ and logs every upgrade it is asked for.
    fake_mise(dir, """
    case "$*" in
      "-C / config ls --json") cat "#{dir}/fake/global.json" ;;
      "ls --all-sources --json") cat "#{dir}/fake/ls.json" ;;
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

    File.write!(Path.join(dir, "fake/ls.json"), JSON.encode!(ls))
    File.write!(Path.join(dir, "fake/pinned.json"), JSON.encode!(pinned))
    File.write!(Path.join(dir, "fake/bumped.json"), "{}")
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

    {out, 0, log} = upgrade(dir, "1 2")

    refute out =~ worktree
    assert log == ["-C #{project} upgrade --no-global"]
  end

  test "leaves out a config under the temporary directory", %{dir: dir} do
    project = Path.join(dir, "project")
    scratch = Path.join(dir, "tmp/scratch")
    File.mkdir_p!(scratch)
    File.write!(Path.join(scratch, "mise.toml"), "")
    tools(dir, [{"jq", "1", "1.6", "1.7", [repo(project), Path.join(scratch, "mise.toml")]}])

    {out, 0, log} = upgrade(dir, "1 2")

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

    {out, 0, log} = upgrade(dir, "1")

    assert out =~ ~r/^1  #{Regex.escape(project)}$/m
    assert log == ["-C #{project} upgrade --no-global"]
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
end

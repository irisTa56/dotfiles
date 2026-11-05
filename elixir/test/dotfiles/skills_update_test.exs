defmodule Dotfiles.SkillsUpdateTest do
  use ExUnit.Case
  import ExUnit.CaptureIO

  alias Dotfiles.FakeCommand

  @root "/work/dotfiles"
  @old "2026-01-01T00:00:00Z"
  @unlisted "GitHub lists no merge into the default branch for its last commit; check by hand"

  @lock [
    %{
      "name" => "old",
      "host" => "github.com",
      "repo_url" => "o/r",
      "virtual_path" => "skills/old"
    },
    %{
      "name" => "recent",
      "host" => "github.com",
      "repo_url" => "o/r",
      "virtual_path" => "skills/recent"
    },
    %{"name" => "own", "host" => "gist.github.com", "repo_url" => "me/abc"}
  ]

  # `github` answers for each skill path, as what gh's --jq filter would print for
  # its last commit: a date for a pull request merged then into the default branch,
  # nil for none listed, :fail for a request that fails, and :no_commit for a path
  # whose commits list comes back empty. `fail` names apm subcommands that fail, and
  # `lock` the lockfile's dependencies, or :fail for a lockfile yq cannot read.
  defp install(github, opts \\ []) do
    fail = Keyword.get(opts, :fail, %{})
    lock = Keyword.get(opts, :lock, @lock)

    FakeCommand.install(fn
      "yq", ["-o=json", "apm.lock.yaml"], _ ->
        if lock == :fail, do: {"", 3}, else: {JSON.encode!(%{"dependencies" => lock}), 0}

      "gh", ["api", path, "--jq", _], _ ->
        gh(github, path)

      "apm", [subcommand | _], _ ->
        {"", Map.get(fail, subcommand, 0)}
    end)
  end

  defp gh(github, path) do
    case Regex.run(
           ~r{^repos/[^/]+/[^/]+/commits(?:\?(?:path=(.*)&)?per_page=1|/sha-(.*)/pulls)$},
           path
         ) do
      [_, skill_path] ->
        if github[skill_path] == :no_commit, do: {"\n", 0}, else: {"sha-#{skill_path}\n", 0}

      [_, "", skill_path] ->
        case github[skill_path] do
          :fail -> {"", 1}
          nil -> {"\n", 0}
          date -> {date <> "\n", 0}
        end
    end
  end

  # The exit status, as elixir's CLI would set it, and stdout followed by stderr.
  defp run(args \\ []) do
    {{code, out}, err} =
      with_io(:stderr, fn ->
        with_io(fn -> status(args, @root) end)
      end)

    {out <> err, code}
  end

  defp status(args, root) do
    Dotfiles.SkillsUpdate.main(args, root)
    0
  catch
    :exit, {:shutdown, code} -> code
  end

  defp apm_calls, do: Enum.map(FakeCommand.calls("apm"), &Enum.join(&1, " "))

  defp hours_ago(hours),
    do: DateTime.utc_now() |> DateTime.add(-hours * 3600) |> DateTime.to_iso8601()

  test "updates a skill whose last change merged over a day ago, and holds a newer one" do
    recent = hours_ago(23)
    install(%{"skills/old" => @old, "skills/recent" => recent})

    {out, 0} = run()

    assert "recent: held, last change merged #{recent}" in String.split(out, "\n")
    assert apm_calls() == ["update --yes o/r/skills/old own", "install", "audit"]
  end

  test "updates a skill just past a day" do
    install(%{"skills/old" => @old, "skills/recent" => hours_ago(25)})

    {_, 0} = run()

    assert hd(apm_calls()) == "update --yes o/r/skills/old o/r/skills/recent own"
  end

  test "runs every apm call in the root it is given" do
    install(%{"skills/old" => @old, "skills/recent" => @old})

    run()

    assert for({"apm", _, opts} <- FakeCommand.calls(), do: opts[:cd]) == List.duplicate(@root, 3)
    assert [{"yq", _, opts}] = for(call = {"yq", _, _} <- FakeCommand.calls(), do: call)
    assert opts[:cd] == @root
  end

  test "asks GitHub nothing about a gist, and updates it" do
    install(%{"skills/old" => @old, "skills/recent" => @old})

    {_, 0} = run()

    refute Enum.any?(FakeCommand.calls("gh"), fn [_, path | _] -> path =~ "me/abc" end)
    assert hd(apm_calls()) =~ ~r/ own$/
  end

  test "asks about a whole repository by its commits, and updates it by name" do
    whole = %{"name" => "whole", "host" => "github.com", "repo_url" => "o/whole"}

    FakeCommand.install(fn
      "yq", _, _ -> {JSON.encode!(%{"dependencies" => [whole]}), 0}
      "gh", ["api", "repos/o/whole/commits?per_page=1" | _], _ -> {"sha-whole\n", 0}
      "gh", ["api", "repos/o/whole/commits/sha-whole/pulls" | _], _ -> {@old <> "\n", 0}
      "apm", _, _ -> {"", 0}
    end)

    {_, 0} = run()

    assert hd(apm_calls()) == "update --yes whole"
  end

  test "holds a path with no commit, and the reason says so" do
    install(%{"skills/old" => @old, "skills/recent" => :no_commit})

    {out, 0} = run()

    assert "recent: held, no commit on this path" in String.split(out, "\n")
    assert hd(apm_calls()) == "update --yes o/r/skills/old own"
  end

  test "holds a commit GitHub lists no merge into the default branch for, with a reason that claims no more" do
    install(%{"skills/old" => @old, "skills/recent" => nil})

    {out, 0} = run()

    assert "recent: held, #{@unlisted}" in String.split(out, "\n")
    assert hd(apm_calls()) == "update --yes o/r/skills/old own"
  end

  test "holds a skill whose request fails rather than updating it" do
    install(%{"skills/old" => @old, "skills/recent" => :fail})

    {out, 0} = run()

    assert "recent: held, GitHub request failed (exit 1)" in String.split(out, "\n")
    assert hd(apm_calls()) == "update --yes o/r/skills/old own"
  end

  test "asks about and updates only the packages named" do
    install(%{"skills/recent" => @old})

    {_, 0} = run(["o/r/skills/recent"])

    assert Enum.map(FakeCommand.calls("gh"), &Enum.at(&1, 1)) == [
             "repos/o/r/commits?path=skills/recent&per_page=1",
             "repos/o/r/commits/sha-skills/recent/pulls"
           ]

    assert hd(apm_calls()) == "update --yes o/r/skills/recent"
  end

  test "refuses a name the lockfile does not hold, asking and updating nothing" do
    install(%{})

    {out, 1} = run(["o/r/skills/typo", "o/r/skills/old"])

    assert out =~ "Not in apm.lock.yaml: o/r/skills/typo\n"
    assert FakeCommand.calls("gh") == []
    assert apm_calls() == []
  end

  test "runs no apm when every chosen skill is held" do
    install(%{"skills/recent" => hours_ago(1)})

    {out, 0} = run(["o/r/skills/recent"])

    assert out =~ "Nothing to update."
    assert apm_calls() == []
  end

  test "fails with apm's status when a step fails, running none after it" do
    install(%{"skills/old" => @old, "skills/recent" => @old}, fail: %{"install" => 4})

    {out, 4} = run()

    assert out =~ "apm install failed (exit 4)"
    assert Enum.map(FakeCommand.calls("apm"), &hd/1) == ["update", "install"]
  end

  test "fails with yq's status when it cannot read the lockfile" do
    install(%{}, lock: :fail)

    {out, 3} = run()

    assert out =~ "yq could not read apm.lock.yaml (exit 3)"
  end

  test "prints its usage for --help and -h, and refuses another flag with exit 2" do
    install(%{})

    for flag <- ["--help", "-h"] do
      {out, 0} = run([flag])
      assert out =~ "Usage: mise run skills:update"
    end

    {out, 2} = run(["--yes"])
    assert out =~ "Usage: mise run skills:update"
    assert FakeCommand.calls() == []
  end
end

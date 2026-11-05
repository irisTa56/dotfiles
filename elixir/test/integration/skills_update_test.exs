# Runs the module against fake apm and gh shell scripts, with the real yq
# reading the lockfile and the real jq applying gh's --jq filter: the unit tests
# answer in place of both, so what they read and the filters are checked here.
defmodule Dotfiles.Integration.SkillsUpdateTest do
  use ExUnit.Case
  @moduletag :integration
  import ExUnit.CaptureIO

  @old "2026-01-01T00:00:00Z"

  setup do
    Dotfiles.TestEnv.real_commands()

    dir = Path.join(System.tmp_dir!(), "skills-update-test-#{System.unique_integer([:positive])}")
    on_exit(fn -> File.rm_rf!(dir) end)
    File.mkdir_p!(Path.join(dir, "bin"))

    File.write!(Path.join(dir, "apm.lock.yaml"), """
    dependencies:
    - name: old
      host: github.com
      repo_url: o/r
      virtual_path: skills/old
    - name: recent
      host: github.com
      repo_url: o/r
      virtual_path: skills/recent
    - name: own
      host: gist.github.com
      repo_url: me/abc
    """)

    fake(dir, "apm", """
    echo "$*" >> "#{dir}/apm.log"
    pwd -P >> "#{dir}/apm.cwd"
    """)

    {:ok, dir: dir}
  end

  defp fake(dir, name, body) do
    path = Path.join([dir, "bin", name])
    File.write!(path, "#!/bin/sh\n" <> body)
    File.chmod!(path, 0o755)
  end

  # Answers for each skill, as the pull request GitHub lists for its last commit: nil for
  # a commit with no merged pull request, a date for a pull request merged then into the
  # default branch, and {:other_branch, date} for one
  # merged then into another branch. :no_commit is a path whose commits list comes back
  # empty, as when upstream moved or deleted it, so no pull request is asked about. The
  # fake gh serves JSON and applies the --jq filter it is given with jq, as gh does.
  defp github(dir, merged) do
    File.mkdir_p!(Path.join(dir, "gh"))

    cases =
      Enum.map_join(merged, fn {skill, answer} ->
        commits = if answer == :no_commit, do: "[]", else: ~s([{"sha": "sha-#{skill}"}])
        File.write!(Path.join(dir, "gh/#{skill}-commits.json"), commits)

        pulls =
          case answer do
            :no_commit ->
              ""

            answer ->
              File.write!(Path.join(dir, "gh/#{skill}-pulls.json"), pulls_json(answer))

              ~s[  */commits/sha-#{skill}/pulls*) jq -r "$4" "#{dir}/gh/#{skill}-pulls.json" ;;\n]
          end

        """
          *path=skills/#{skill}\\&*) jq -r "$4" "#{dir}/gh/#{skill}-commits.json" ;;
        """ <> pulls
      end)

    fake(dir, "gh", "case \"$2\" in\n#{cases}  *) exit 1 ;;\nesac\n")
  end

  defp pulls_json(nil), do: "[]"
  defp pulls_json({:other_branch, date}), do: pulls_json(date, "dev")
  defp pulls_json(date), do: pulls_json(date, "main")

  defp pulls_json(date, base) do
    JSON.encode!([
      %{
        "merged_at" => date,
        "base" => %{"ref" => base, "repo" => %{"default_branch" => "main"}}
      }
    ])
  end

  defp run(dir) do
    tool_dirs =
      for tool <- ["yq", "jq"] do
        {bin, 0} = System.cmd("mise", ["which", tool])
        Path.dirname(String.trim(bin))
      end

    path = Enum.join([Path.join(dir, "bin") | tool_dirs] ++ [System.get_env("PATH")], ":")
    Dotfiles.TestEnv.put_env([{"PATH", path}])

    {{code, out}, err} =
      with_io(:stderr, fn ->
        with_io(fn -> status(fn -> Dotfiles.SkillsUpdate.main([], dir) end) end)
      end)

    {out <> err, code}
  end

  # The exit status a run ends with, as elixir's CLI would set it.
  defp status(fun) do
    fun.()
    0
  catch
    :exit, {:shutdown, code} -> code
  end

  defp apm_calls(dir) do
    case File.read(Path.join(dir, "apm.log")) do
      {:ok, log} -> String.split(log, "\n", trim: true)
      _ -> []
    end
  end

  defp hours_ago(hours) do
    DateTime.utc_now() |> DateTime.add(-hours * 3600) |> DateTime.to_iso8601()
  end

  test "a skill changed over a day ago updates, and a newer one is held", %{dir: dir} do
    github(dir, %{"old" => @old, "recent" => hours_ago(23)})

    {out, code} = run(dir)

    assert code == 0, out
    assert out =~ "recent: held"
    assert "update --yes o/r/skills/old own" in apm_calls(dir)
  end

  test "a path with no commit is held, and the reason says so", %{dir: dir} do
    github(dir, %{"old" => @old, "recent" => :no_commit})

    {out, code} = run(dir)

    assert code == 0, out
    assert "recent: held, no commit on this path" in String.split(out, "\n")
    assert "update --yes o/r/skills/old own" in apm_calls(dir)
  end

  @unlisted "GitHub lists no merge into the default branch for its last commit; check by hand"

  test "a commit with no pull request listed is held with a reason that claims no more", %{
    dir: dir
  } do
    github(dir, %{"old" => @old, "recent" => nil})

    {out, code} = run(dir)

    assert code == 0, out
    assert "recent: held, #{@unlisted}" in String.split(out, "\n")
    assert "update --yes o/r/skills/old own" in apm_calls(dir)
  end

  test "a commit whose only pull request went to another branch is held with the same reason",
       %{dir: dir} do
    github(dir, %{"old" => @old, "recent" => {:other_branch, @old}})

    {out, code} = run(dir)

    assert code == 0, out
    assert "recent: held, #{@unlisted}" in String.split(out, "\n")
    assert "update --yes o/r/skills/old own" in apm_calls(dir)
  end

  # The task itself runs from the Mix project, so apm reaches the repository's
  # apm.yml only through the root it is given.
  test "every apm call runs in the root it is given", %{dir: dir} do
    github(dir, %{"old" => @old, "recent" => @old})

    run(dir)

    {root, 0} = System.cmd("pwd", ["-P"], cd: dir)
    cwds = dir |> Path.join("apm.cwd") |> File.read!() |> String.split("\n", trim: true)
    assert length(cwds) == 3
    assert Enum.all?(cwds, &(&1 == String.trim(root)))
  end
end

#!/usr/bin/env elixir

ExUnit.start()

defmodule SkillsUpdateTest do
  use ExUnit.Case

  @script Path.expand("skills_update.exs", __DIR__)
  @old "2026-01-01T00:00:00Z"

  setup do
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
    [ "$1" = audit ] && exit "${APM_AUDIT:-0}"
    exit 0
    """)

    {:ok, dir: dir}
  end

  defp fake(dir, name, body) do
    path = Path.join([dir, "bin", name])
    File.write!(path, "#!/bin/sh\n" <> body)
    File.chmod!(path, 0o755)
  end

  # Answers for each skill, as the time its last commit's pull request merged: nil for
  # a commit with no merged pull request, and :fail for a request that fails.
  defp github(dir, merged) do
    cases =
      Enum.map_join(merged, fn {skill, answer} ->
        pulls =
          case answer do
            :fail -> "exit 1"
            nil -> ":"
            date -> "echo #{date}"
          end

        """
          *path=skills/#{skill}\\&*) echo sha-#{skill} ;;
          */commits/sha-#{skill}/pulls*) #{pulls} ;;
        """
      end)

    fake(dir, "gh", "case \"$2\" in\n#{cases}  *) exit 1 ;;\nesac\n")
  end

  defp run(dir, args \\ [], env \\ []) do
    {yq, 0} = System.cmd("mise", ["which", "yq"])

    path =
      Enum.join(
        [Path.join(dir, "bin"), Path.dirname(String.trim(yq)), System.get_env("PATH")],
        ":"
      )

    System.cmd("elixir", [@script | args],
      cd: dir,
      env: [{"PATH", path} | env],
      stderr_to_stdout: true
    )
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

  test "a skill just past a day updates", %{dir: dir} do
    github(dir, %{"old" => @old, "recent" => hours_ago(25)})

    {out, code} = run(dir)

    assert code == 0, out
    assert "update --yes o/r/skills/old o/r/skills/recent own" in apm_calls(dir)
  end

  test "a commit with no merged pull request is held", %{dir: dir} do
    github(dir, %{"old" => @old, "recent" => nil})

    {out, _} = run(dir)

    assert out =~ "recent: held"
    assert "update --yes o/r/skills/old own" in apm_calls(dir)
  end

  test "a failed request holds the skill rather than updating it", %{dir: dir} do
    github(dir, %{"old" => @old, "recent" => :fail})

    {out, _} = run(dir)

    assert out =~ "recent: held"
    assert "update --yes o/r/skills/old own" in apm_calls(dir)
  end

  test "an install follows the update and the audit follows that", %{dir: dir} do
    github(dir, %{"old" => @old, "recent" => @old})

    run(dir)

    assert [
             "update --yes o/r/skills/old o/r/skills/recent own",
             "install",
             "audit"
           ] = apm_calls(dir)
  end

  test "named packages are the only ones asked about and updated", %{dir: dir} do
    github(dir, %{"recent" => @old})

    {out, code} = run(dir, ["o/r/skills/recent"])

    assert code == 0, out
    assert "update --yes o/r/skills/recent" in apm_calls(dir)
  end

  test "a name the lockfile does not hold is refused", %{dir: dir} do
    github(dir, %{"old" => @old, "recent" => @old})

    {out, code} = run(dir, ["o/r/skills/typo"])

    assert code != 0, out
    assert out =~ "o/r/skills/typo"
    assert apm_calls(dir) == []
  end

  test "with every named skill held apm updates nothing", %{dir: dir} do
    github(dir, %{"recent" => hours_ago(1)})

    {out, code} = run(dir, ["o/r/skills/recent"])

    assert code == 0, out
    refute Enum.any?(apm_calls(dir), &String.starts_with?(&1, "update"))
  end

  test "a failed audit fails the run", %{dir: dir} do
    github(dir, %{"old" => @old, "recent" => @old})

    {_, code} = run(dir, [], [{"APM_AUDIT", "1"}])

    assert code != 0
  end
end

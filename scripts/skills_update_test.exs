#!/usr/bin/env elixir

ExUnit.start()

defmodule SkillsUpdateCheckTest do
  use ExUnit.Case

  @script Path.expand("skills_update.exs", __DIR__)
  @skill "---\nname: probe\ndescription: d\n---\nbody\n"

  setup do
    dir = Path.join(System.tmp_dir!(), "skills-update-test-#{System.unique_integer([:positive])}")
    on_exit(fn -> File.rm_rf!(dir) end)
    File.mkdir_p!(dir)
    {:ok, dir: dir}
  end

  defp write(dir, path, content) do
    full = Path.join(dir, path)
    File.mkdir_p!(Path.dirname(full))
    File.write!(full, content)
  end

  # A lockfile deploying the skill, or `paths` where the test names its own.
  defp lock(dir, paths \\ [".claude/skills/probe", ".claude/skills/probe/SKILL.md"]) do
    listed = Enum.map_join(paths, "", &"  - #{&1}\n")
    write(dir, "apm.lock.yaml", "dependencies:\n- name: probe\n  deployed_files:\n#{listed}")
  end

  # The fixture sits outside this repository, where mise's shim finds no yq version.
  defp check(dir) do
    {yq, 0} = System.cmd("mise", ["which", "yq"])
    path = Path.dirname(String.trim(yq)) <> ":" <> System.get_env("PATH")

    System.cmd("elixir", [@script, "--check"],
      cd: dir,
      env: [{"PATH", path}],
      stderr_to_stdout: true
    )
  end

  defp assert_flagged(dir, text) do
    {out, code} = check(dir)
    assert code == 1, out
    assert out =~ text
  end

  defp assert_clean(dir) do
    {out, code} = check(dir)
    assert code == 0, out
  end

  test "a skill of plain instructions passes", %{dir: dir} do
    lock(dir)
    write(dir, ".claude/skills/probe/SKILL.md", @skill)
    write(dir, ".claude/skills/probe/scripts/run.py", "print()\n")

    assert_clean(dir)
  end

  for path <- [".claude-plugin/plugin.json", "hooks/hooks.json", ".mcp.json", "bin/tool"] do
    test "#{path} in a skill is flagged", %{dir: dir} do
      lock(dir)
      write(dir, ".claude/skills/probe/SKILL.md", @skill)
      write(dir, ".claude/skills/probe/#{unquote(path)}", "{}\n")

      assert_flagged(dir, ".claude/skills/probe/#{unquote(path)}")
    end
  end

  for key <- ["hooks", "allowed-tools"] do
    test "#{key} in the frontmatter is flagged", %{dir: dir} do
      lock(dir)

      write(
        dir,
        ".claude/skills/probe/SKILL.md",
        "---\nname: probe\n#{unquote(key)}: x\n---\nbody\n"
      )

      assert_flagged(dir, unquote(key))
    end
  end

  test "the same words in the body pass", %{dir: dir} do
    lock(dir)
    write(dir, ".claude/skills/probe/SKILL.md", @skill <> "hooks: none\nallowed-tools: none\n")

    assert_clean(dir)
  end

  for {name, body} <- [
        {"at the start of a line", "!`id`\n"},
        {"after whitespace", "- user: !`id`\n"},
        {"as a fence", "```!\nid\n```\n"}
      ] do
    test "a command #{name} is flagged", %{dir: dir} do
      lock(dir)
      write(dir, ".claude/skills/probe/SKILL.md", @skill <> unquote(body))

      assert_flagged(dir, "command")
    end
  end

  test "a placeholder that stays literal passes", %{dir: dir} do
    lock(dir)
    write(dir, ".claude/skills/probe/SKILL.md", @skill <> "KEY=!`id`\n")

    assert_clean(dir)
  end

  test "a file deployed outside the skills directory is flagged", %{dir: dir} do
    lock(dir, [".claude/rules/probe.md"])
    write(dir, ".claude/rules/probe.md", "rule\n")

    assert_flagged(dir, ".claude/rules/probe.md")
  end

  test "a skill the lockfile does not deploy is left alone", %{dir: dir} do
    lock(dir)
    write(dir, ".claude/skills/probe/SKILL.md", @skill)
    write(dir, ".claude/skills/global-hooks/.claude-plugin/plugin.json", "{}\n")

    assert_clean(dir)
  end
end

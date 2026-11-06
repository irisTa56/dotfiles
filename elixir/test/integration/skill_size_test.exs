# Runs the module against a real git repository, since what git tracks is what
# the check is owed to cover.
defmodule Dotfiles.Integration.SkillSizeTest do
  use ExUnit.Case
  @moduletag :integration
  import ExUnit.CaptureIO

  setup do
    Dotfiles.TestEnv.real_commands()
    root = Dotfiles.TestEnv.tmp_dir!("skill-size-test")

    # A git hook running this suite sets GIT_DIR and the like for its own
    # repository, which would turn every git call here toward that one: `git init`
    # there marks the repository bare.
    inherited =
      for {name, _} <- System.get_env(), String.starts_with?(name, "GIT_"), do: {name, nil}

    Dotfiles.TestEnv.put_env([{"GIT_CEILING_DIRECTORIES", root} | inherited])

    {:ok, root: root}
  end

  defp git(root, args) do
    {out, 0} = System.cmd("git", args, cd: root, stderr_to_stdout: true)
    out
  end

  test "an over-limit skill that git tracks through an unslashed unignore fails the run", %{
    root: root
  } do
    git(root, ["init", "-q"])
    File.write!(Path.join(root, ".gitignore"), "/.claude/skills/*\n!/.claude/skills/foo\n")

    dir = Path.join([root, ".claude/skills/foo"])
    File.mkdir_p!(dir)

    File.write!(
      Path.join(dir, "SKILL.md"),
      "---\nname: foo\ndescription: d\n---\n" <> String.duplicate("a", 19_801)
    )

    git(root, ["add", "-A"])
    assert git(root, ["ls-files"]) =~ ".claude/skills/foo/SKILL.md"

    {{code, _out}, err} =
      with_io(:stderr, fn ->
        with_io(fn ->
          try do
            Dotfiles.SkillSize.main([], root)
            0
          catch
            :exit, {:shutdown, code} -> code
          end
        end)
      end)

    assert code != 0
    # Not the exit of the no-skill-found guard: foo itself must be reported.
    assert err =~ "foo: 19801"
  end

  # Vendored skills are restored from upstream on every install, so their length
  # is not this repository's to keep; a directory with no SKILL.md is no skill.
  test "leaves out an ignored skill and a tracked directory with no SKILL.md", %{root: root} do
    git(root, ["init", "-q"])

    File.write!(
      Path.join(root, ".gitignore"),
      "/.claude/skills/*\n!/.claude/skills/own/\n!/.claude/skills/plugin/\n"
    )

    for {name, body} <- [{"own", "a"}, {"vendored", String.duplicate("a", 19_801)}] do
      dir = Path.join([root, ".claude/skills", name])
      File.mkdir_p!(dir)
      File.write!(Path.join(dir, "SKILL.md"), "---\nname: #{name}\ndescription: d\n---\n" <> body)
    end

    File.mkdir_p!(Path.join(root, ".claude/skills/plugin"))
    File.write!(Path.join(root, ".claude/skills/plugin/hooks.json"), "{}")
    git(root, ["add", "-A"])

    assert capture_io(:stderr, fn -> Dotfiles.SkillSize.main([], root) end) == ""
  end
end

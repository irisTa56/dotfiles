defmodule Dotfiles.SkillSizeTest do
  use ExUnit.Case
  import ExUnit.CaptureIO

  alias Dotfiles.FakeCommand

  @limit 19_800

  # A repository root holding each skill in `skills` as {name, SKILL.md content},
  # every one of which git answers as tracked.
  defp repo(skills) do
    root = Dotfiles.TestEnv.tmp_dir!("skill-size")

    for {name, content} <- skills do
      dir = Path.join([root, ".claude/skills", name])
      File.mkdir_p!(dir)
      File.write!(Path.join(dir, "SKILL.md"), content)
    end

    listed = Enum.map_join(skills, fn {name, _} -> ".claude/skills/#{name}/SKILL.md\0" end)
    FakeCommand.install(fn "git", ["-C", ^root, "ls-files" | _], _ -> {listed, 0} end)
    root
  end

  defp skill(body), do: "---\nname: x\ndescription: y\n---\n" <> body

  # The exit status, as elixir's CLI would set it, and stdout followed by stderr.
  defp run(root) do
    {{code, out}, err} =
      with_io(:stderr, fn ->
        with_io(fn -> status(root) end)
      end)

    {out <> err, code}
  end

  defp status(root) do
    Dotfiles.SkillSize.main([], root)
    0
  catch
    :exit, {:shutdown, code} -> code
  end

  test "reports a tracked skill whose body is past the limit, and fails" do
    root = repo([{"big", skill(String.duplicate("a", @limit + 1))}])

    {out, code} = run(root)

    assert code == 1
    assert out =~ "big: #{@limit + 1}"
  end

  test "counts a character outside the BMP as the two code units JavaScript does" do
    root = repo([{"emoji", skill(String.duplicate("a", @limit - 1) <> "😀")}])

    {out, code} = run(root)

    assert code == 1
    assert out =~ "emoji: #{@limit + 1}"
  end

  test "passes a body at the limit" do
    root = repo([{"full", skill(String.duplicate("a", @limit))}])

    assert run(root) == {"", 0}
  end

  test "fails when git lists no skill to check" do
    root = repo([])

    {out, code} = run(root)

    assert code == 1
    assert out =~ "git tracks no skill"
  end

  test "fails when git ls-files fails" do
    root = repo([])

    FakeCommand.install(fn "git", _, _ -> {"", 128} end)

    {out, code} = run(root)

    assert code == 1
    assert out =~ "git ls-files failed (exit 128)"
  end
end

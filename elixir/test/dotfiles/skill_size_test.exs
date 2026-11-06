defmodule Dotfiles.SkillSizeTest do
  use ExUnit.Case, async: true
  import ExUnit.CaptureIO

  alias Dotfiles.TestEnv

  @limit 19_800

  # A repository root whose .gitignore unignores `tracked`, holding each skill in
  # `skills` as {name, SKILL.md content}.
  defp repo(tracked, skills) do
    root = TestEnv.tmp_dir!("skill-size")

    File.write!(
      Path.join(root, ".gitignore"),
      Enum.map_join(
        ["/.claude/skills/*" | Enum.map(tracked, &"!/.claude/skills/#{&1}/")],
        "",
        &"#{&1}\n"
      )
    )

    for {name, content} <- skills do
      dir = Path.join([root, ".claude/skills", name])
      File.mkdir_p!(dir)
      File.write!(Path.join(dir, "SKILL.md"), content)
    end

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
    root = repo(["big"], [{"big", skill(String.duplicate("a", @limit + 1))}])

    {out, code} = run(root)

    assert code == 1
    assert out =~ "big: #{@limit + 1}"
  end

  test "counts a character outside the BMP as the two code units JavaScript does" do
    root = repo(["emoji"], [{"emoji", skill(String.duplicate("a", @limit - 1) <> "😀")}])

    {out, code} = run(root)

    assert code == 1
    assert out =~ "emoji: #{@limit + 1}"
  end

  test "passes a tracked directory that holds no SKILL.md, and a body at the limit" do
    root = repo(["plugin", "full"], [{"full", skill(String.duplicate("a", @limit))}])
    File.mkdir_p!(Path.join(root, ".claude/skills/plugin"))

    assert run(root) == {"", 0}
  end

  # Vendored skills are restored from upstream on every install, so their length
  # is not this repository's to keep.
  test "leaves out a skill .gitignore does not unignore" do
    root = repo([], [{"vendored", skill(String.duplicate("a", @limit + 1))}])

    assert run(root) == {"", 0}
  end
end

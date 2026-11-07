# Runs the shared task against a real git repository, since what git tracks is
# what the check is owed to cover.
defmodule Dotfiles.Integration.SkillSizeCheckTest do
  use ExUnit.Case
  @moduletag :integration

  @script Path.expand("../../../tasks/skills/size-check.sh", __DIR__)
  @limit 19_800

  setup do
    root = Dotfiles.TestEnv.tmp_dir!("skill-size-check-test")

    # A git hook running this suite sets GIT_DIR and the like for its own
    # repository, which would turn every git call here toward that one: `git init`
    # there marks the repository bare.
    inherited =
      for {name, _} <- System.get_env(), String.starts_with?(name, "GIT_"), do: {name, nil}

    Dotfiles.TestEnv.put_env([{"GIT_CEILING_DIRECTORIES", root} | inherited])
    git(root, ["init", "-q"])

    {:ok, root: root}
  end

  defp git(root, args) do
    {out, 0} = System.cmd("git", args, cd: root, stderr_to_stdout: true)
    out
  end

  # Writes each {path, SKILL.md content} under root and stages it.
  defp add(root, skills) do
    for {path, content} <- skills do
      file = Path.join(root, path)
      File.mkdir_p!(Path.dirname(file))
      File.write!(file, content)
    end

    git(root, ["add", "-A"])
  end

  defp skill(body), do: "---\nname: x\ndescription: y\n---\n" <> body

  # The task's output, stdout and stderr together, and its exit status.
  defp check(root, args \\ []) do
    System.cmd("bash", [@script | args], cd: root, stderr_to_stdout: true)
  end

  test "reports a tracked skill whose body is past the limit, and fails", %{root: root} do
    add(root, [{".claude/skills/big/SKILL.md", skill(String.duplicate("a", @limit + 1))}])

    {out, code} = check(root)

    assert code == 1
    assert out =~ ".claude/skills/big/SKILL.md: #{@limit + 1}"
  end

  test "counts a character outside the BMP as the two code units JavaScript does", %{
    root: root
  } do
    add(root, [
      {".claude/skills/emoji/SKILL.md", skill(String.duplicate("a", @limit - 1) <> "😀")}
    ])

    {out, code} = check(root)

    assert code == 1
    assert out =~ "emoji/SKILL.md: #{@limit + 1}"
  end

  test "passes a body at the limit, frontmatter not counted", %{root: root} do
    add(root, [{".claude/skills/full/SKILL.md", skill(String.duplicate("a", @limit))}])

    assert check(root) == {"", 0}
  end

  # Run where git lists nothing, the check would otherwise pass having read no skill.
  test "fails when git tracks no skill to check", %{root: root} do
    {out, code} = check(root)

    assert code == 1
    assert out =~ "git tracks no skill"
  end

  test "fails where git cannot list the skills" do
    outside = Dotfiles.TestEnv.tmp_dir!("skill-size-check-outside")
    Dotfiles.TestEnv.put_env([{"GIT_CEILING_DIRECTORIES", outside}])

    {out, code} = check(outside)

    assert code != 0
    assert out =~ "not a git repository"
  end

  # Vendored skills are restored from upstream on every install, so their length
  # is not the repository's to keep; a directory with no SKILL.md is no skill.
  test "leaves out an ignored skill and a tracked directory with no SKILL.md", %{root: root} do
    File.write!(
      Path.join(root, ".gitignore"),
      "/.claude/skills/*\n!/.claude/skills/own/\n!/.claude/skills/plugin/\n"
    )

    add(root, [
      {".claude/skills/own/SKILL.md", skill("a")},
      {".claude/skills/vendored/SKILL.md", skill(String.duplicate("a", @limit + 1))},
      {".claude/skills/plugin/hooks.json", "{}"}
    ])

    assert check(root) == {"", 0}
  end

  test "checks the skills a pathspec argument names in place of .claude/skills", %{root: root} do
    add(root, [
      {"skills/big/SKILL.md", skill(String.duplicate("a", @limit + 1))},
      {".claude/skills/small/SKILL.md", skill("a")}
    ])

    {out, code} = check(root, ["skills/*/SKILL.md"])

    assert code == 1
    assert out =~ "skills/big/SKILL.md: #{@limit + 1}"
  end

  # A skill whose frontmatter the check cannot find is not a short one: it either
  # counts the right part or refuses the file, but never passes it.
  test "does not pass a skill whose frontmatter fences end in CRLF", %{root: root} do
    content = "---\r\nname: x\r\ndescription: y\r\n---\r\n" <> String.duplicate("a", 25_000)
    add(root, [{".claude/skills/crlf/SKILL.md", content}])

    {out, code} = check(root)

    assert code != 0
    assert out =~ ".claude/skills/crlf/SKILL.md"
  end

  test "leaves a CRLF frontmatter out of the count", %{root: root} do
    content = "---\r\nname: x\r\ndescription: y\r\n---\r\n" <> String.duplicate("a", @limit)
    add(root, [{".claude/skills/crlf/SKILL.md", content}])

    assert check(root) == {"", 0}
  end

  test "does not pass a skill with no frontmatter and a body past the limit", %{root: root} do
    add(root, [{".claude/skills/bare/SKILL.md", String.duplicate("a", @limit + 1_000)}])

    {out, code} = check(root)

    assert code != 0
    assert out =~ ".claude/skills/bare/SKILL.md"
  end

  test "does not pass a skill with no frontmatter and a rule line below the limit", %{
    root: root
  } do
    content = String.duplicate("a", @limit + 1_000) <> "\n---\nshort tail\n"
    add(root, [{".claude/skills/rule/SKILL.md", content}])

    {out, code} = check(root)

    assert code != 0
    assert out =~ ".claude/skills/rule/SKILL.md"
  end

  test "fails on a frontmatter that never closes, rather than reading it as all frontmatter",
       %{root: root} do
    add(root, [
      {".claude/skills/open/SKILL.md", "---\nname: x\n" <> String.duplicate("a", @limit + 1)}
    ])

    {out, code} = check(root)

    assert code == 1
    assert out =~ ".claude/skills/open/SKILL.md: its frontmatter has no closing ---"
  end

  test "reports an over-limit skill sorted after one under a non-ASCII directory", %{
    root: root
  } do
    add(root, [
      {".claude/skills/é/SKILL.md", skill("a")},
      {".claude/skills/zz/SKILL.md", skill(String.duplicate("a", @limit + 1))}
    ])

    {out, code} = check(root)

    assert code == 1
    assert out =~ ".claude/skills/zz/SKILL.md: #{@limit + 1}"
  end

  test "passes a short skill under a non-ASCII directory", %{root: root} do
    add(root, [{".claude/skills/é/SKILL.md", skill("a")}])

    assert check(root) == {"", 0}
  end

  # The check reads the working tree, so a tracked skill whose file is gone from
  # disk with the deletion unstaged must not end the run before the others are read.
  test "still reports an over-limit skill when an earlier tracked one is deleted from disk", %{
    root: root
  } do
    add(root, [
      {".claude/skills/aa/SKILL.md", skill("a")},
      {".claude/skills/zz/SKILL.md", skill(String.duplicate("a", @limit + 1))}
    ])

    File.rm!(Path.join(root, ".claude/skills/aa/SKILL.md"))

    {out, code} = check(root)

    assert code == 1
    assert out =~ ".claude/skills/zz/SKILL.md: #{@limit + 1}"
  end

  # A bare path and number leave the reader of a failed commit or CI run to open
  # the script to learn what the number counts and what it is held to.
  test "names the limit and the unit beside the path and length of an over-limit skill", %{
    root: root
  } do
    add(root, [{".claude/skills/big/SKILL.md", skill(String.duplicate("a", @limit + 1))}])

    {out, code} = check(root)

    assert code == 1
    assert out =~ ".claude/skills/big/SKILL.md"
    assert out =~ Integer.to_string(@limit + 1)
    assert out =~ Integer.to_string(@limit)
    assert out =~ "UTF-16 code units"
  end
end

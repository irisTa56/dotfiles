# After a compaction, Claude Code re-attaches each skill the session invoked, cut
# short: its docs say to the first 5,000 tokens, and session transcripts show the
# cut at 20,000 characters. A rule past the cut is gone from the re-attached copy,
# so a skill tracked here keeps its body under it and sends what it needs less
# often to files it reads by path. Length is counted in UTF-16 code units, as
# JavaScript counts a string's, which is never fewer than its characters.
defmodule Dotfiles.SkillSize do
  # The re-attached copy opens with a line naming the skill's directory, whose
  # length depends on where the skill was loaded from.
  @limit 20_000 - 200

  # `root` is the repository whose tracked skills are checked.
  def main([], root) do
    over =
      for name <- tracked_skills(root),
          length = body_length(root, name),
          length > @limit,
          do: {name, length}

    if over != [] do
      IO.puts(:stderr, "Skill bodies past #{@limit} UTF-16 code units, which a compaction cuts:")
      Enum.each(over, fn {name, length} -> IO.puts(:stderr, "#{name}: #{length}") end)
      exit({:shutdown, 1})
    end
  end

  # The skills this repository tracks are the ones .gitignore unignores; one of
  # those directories may hold a plugin rather than a skill.
  defp tracked_skills(root) do
    root
    |> Path.join(".gitignore")
    |> File.read!()
    |> String.split("\n")
    |> Enum.flat_map(fn line ->
      case Regex.run(~r{^!/\.claude/skills/([^/]+)/$}, line) do
        [_, name] -> [name]
        nil -> []
      end
    end)
    |> Enum.filter(&File.exists?(skill_md(root, &1)))
  end

  defp skill_md(root, name), do: Path.join([root, ".claude/skills", name, "SKILL.md"])

  # The copy holds the body without its frontmatter.
  defp body_length(root, name) do
    [_, _frontmatter, body] =
      root |> skill_md(name) |> File.read!() |> String.split("---\n", parts: 3)

    body |> :unicode.characters_to_binary(:utf8, :utf16) |> byte_size() |> div(2)
  end
end

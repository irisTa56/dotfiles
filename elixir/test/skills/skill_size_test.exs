# After a compaction, Claude Code re-attaches each skill the session invoked, cut
# short: its docs say to the first 5,000 tokens, and session transcripts show the
# cut at 20,000 characters. A rule past the cut is gone from the re-attached copy,
# so a skill tracked here keeps its body under it and sends what it needs less
# often to files it reads by path. Length is counted in UTF-16 code units, as
# JavaScript counts a string's, which is never fewer than its characters.
defmodule Dotfiles.SkillSizeTest do
  use ExUnit.Case, async: true

  @root Path.expand("../../..", __DIR__)

  # The re-attached copy opens with a line naming the skill's directory, whose
  # length depends on where the skill was loaded from.
  @limit 20_000 - 200

  # The skills this repository tracks are the ones .gitignore unignores.
  defp tracked_skills do
    @root
    |> Path.join(".gitignore")
    |> File.read!()
    |> String.split("\n")
    |> Enum.flat_map(fn line ->
      case Regex.run(~r{^!/\.claude/skills/([^/]+)/$}, line) do
        [_, name] -> [name]
        nil -> []
      end
    end)
    |> Enum.filter(&File.exists?(skill_md(&1)))
  end

  defp skill_md(name), do: Path.join([@root, ".claude/skills", name, "SKILL.md"])

  # The copy holds the body without its frontmatter.
  defp body_length(name) do
    [_, _frontmatter, body] =
      name |> skill_md() |> File.read!() |> String.split("---\n", parts: 3)

    body |> :unicode.characters_to_binary(:utf8, :utf16) |> byte_size() |> div(2)
  end

  test "finds the tracked skills" do
    assert "review-loop" in tracked_skills()
  end

  test "each tracked skill's body fits in what a compaction re-attaches" do
    over = for name <- tracked_skills(), body_length(name) > @limit, do: {name, body_length(name)}
    assert over == []
  end
end

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
    # Run where git lists nothing, as from a directory that is not this
    # repository, the check would otherwise pass having read no skill.
    skills = tracked_skills(root)

    if skills == [] do
      IO.puts(:stderr, "git tracks no skill under #{root} to check.")
      exit({:shutdown, 1})
    end

    over =
      for name <- skills,
          length = body_length(root, name),
          length > @limit,
          do: {name, length}

    if over != [] do
      IO.puts(:stderr, "Skill bodies past #{@limit} UTF-16 code units, which a compaction cuts:")
      Enum.each(over, fn {name, length} -> IO.puts(:stderr, "#{name}: #{length}") end)
      exit({:shutdown, 1})
    end
  end

  # Asked of git rather than read off .gitignore, whose unignore lines can take
  # more than one form. The index counts, so a skill staged for the commit being
  # made is checked. `glob` keeps `*` to one directory level.
  defp tracked_skills(root) do
    # git's own message reaches stderr directly, and stays out of the list.
    case Dotfiles.Command.cmd("git", [
           "-C",
           root,
           "ls-files",
           "-z",
           "--",
           ":(glob).claude/skills/*/SKILL.md"
         ]) do
      {out, 0} ->
        for path <- String.split(out, <<0>>, trim: true),
            do: path |> Path.dirname() |> Path.basename()

      {_, code} ->
        IO.puts(:stderr, "git ls-files failed (exit #{code})")
        exit({:shutdown, 1})
    end
  end

  defp skill_md(root, name), do: Path.join([root, ".claude/skills", name, "SKILL.md"])

  # The copy holds the body without its frontmatter.
  defp body_length(root, name) do
    [_, _frontmatter, body] =
      root |> skill_md(name) |> File.read!() |> String.split("---\n", parts: 3)

    body |> :unicode.characters_to_binary(:utf8, :utf16) |> byte_size() |> div(2)
  end
end

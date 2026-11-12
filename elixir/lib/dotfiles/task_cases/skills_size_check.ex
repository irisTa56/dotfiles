# What the shared task skills:size-check must do, each case a repository it runs in.
defmodule Dotfiles.TaskCases.SkillsSizeCheck do
  @task "tasks/skills/size-check.sh"
  @limit 19_800

  defp skill(body), do: "---\nname: x\ndescription: y\n---\n" <> body
  defp over, do: String.duplicate("a", @limit + 1)
  defp crlf(body), do: "---\r\nname: x\r\ndescription: y\r\n---\r\n" <> body

  defp over_limit(path), do: [path <> ": #{@limit + 1}"]

  def cases do
    [
      %{
        name: "reports a tracked skill whose body is past the limit, with the limit and unit",
        files: [{".claude/skills/big/SKILL.md", skill(over())}],
        exit: 1,
        output: [".claude/skills/big/SKILL.md: #{@limit + 1}", "UTF-16 code units", "#{@limit}"]
      },
      %{
        name: "counts a character outside the BMP as the two code units JavaScript does",
        files: [{".claude/skills/e/SKILL.md", skill(String.duplicate("a", @limit - 1) <> "😀")}],
        exit: 1,
        output: over_limit(".claude/skills/e/SKILL.md")
      },
      %{
        name: "passes a body at the limit, frontmatter not counted",
        files: [{".claude/skills/full/SKILL.md", skill(String.duplicate("a", @limit))}],
        exit: 0,
        output: ""
      },
      # A session sizing a skill against the limit needs the length of one that passes.
      %{
        name: "with --verbose, reports a passing skill's length against the limit",
        files: [{".claude/skills/full/SKILL.md", skill(String.duplicate("a", @limit))}],
        args: ["--verbose"],
        exit: 0,
        output: ".claude/skills/full/SKILL.md: #{@limit} of #{@limit} UTF-16 code units\n"
      },
      %{
        name: "with --verbose and a pathspec, reports the passing skills and still fails",
        files: [
          {"skills/big/SKILL.md", skill(over())},
          {"skills/small/SKILL.md", skill("a")}
        ],
        args: ["--verbose", ":(glob)skills/*/SKILL.md"],
        exit: 1,
        output: over_limit("skills/big/SKILL.md") ++ ["skills/small/SKILL.md: 1 of #{@limit}"]
      },
      # An argument after the pathspec is not read, and dropping it silently would
      # leave the caller with no lengths and no reason.
      %{
        name: "fails naming an argument it does not read, rather than dropping it",
        files: [{".claude/skills/small/SKILL.md", skill("a")}],
        args: [":(glob).claude/skills/*/SKILL.md", "--verbose"],
        exit: 2,
        output: ["--verbose"]
      },
      # Run where git lists nothing, the check would otherwise pass having read no skill.
      %{
        name: "fails when git tracks no skill to check",
        files: [],
        exit: 1,
        output: ["git tracks no skill"]
      },
      %{
        name: "fails where git cannot list the skills",
        repository: false,
        files: [],
        exit: 128,
        output: ["not a git repository"]
      },
      # Vendored skills are restored from upstream on every install, so their length
      # is not the repository's to keep; a directory with no SKILL.md is no skill.
      %{
        name: "leaves out an ignored skill and a tracked directory with no SKILL.md",
        files: [
          {".gitignore", "/.claude/skills/*\n!/.claude/skills/own/\n!/.claude/skills/plugin/\n"},
          {".claude/skills/own/SKILL.md", skill("a")},
          {".claude/skills/vendored/SKILL.md", skill(over())},
          {".claude/skills/plugin/hooks.json", "{}"}
        ],
        exit: 0,
        output: ""
      },
      %{
        name: "checks only the SKILL.md one level under .claude/skills",
        files: [
          {".claude/skills/x/SKILL.md", skill("a")},
          {".claude/skills/x/fixtures/y/SKILL.md", skill(over())}
        ],
        exit: 0,
        output: ""
      },
      %{
        name: "checks the skills a pathspec argument names in place of .claude/skills",
        files: [
          {"skills/big/SKILL.md", skill(over())},
          {".claude/skills/small/SKILL.md", skill("a")}
        ],
        args: [":(glob)skills/*/SKILL.md"],
        exit: 1,
        output: over_limit("skills/big/SKILL.md")
      },
      %{
        name: "leaves a CRLF frontmatter out of the count",
        files: [{".claude/skills/crlf/SKILL.md", crlf(String.duplicate("a", @limit))}],
        exit: 0,
        output: ""
      },
      %{
        name: "reports a skill past the limit behind a CRLF frontmatter",
        files: [{".claude/skills/crlf/SKILL.md", crlf(over())}],
        exit: 1,
        output: over_limit(".claude/skills/crlf/SKILL.md")
      },
      # With no frontmatter, the whole file is the body: a `---` rule in it is no fence.
      %{
        name: "counts the whole of a skill with no frontmatter",
        files: [{".claude/skills/bare/SKILL.md", over()}],
        exit: 1,
        output: over_limit(".claude/skills/bare/SKILL.md")
      },
      %{
        name: "counts the whole of a skill with no frontmatter and a rule line in it",
        files: [{".claude/skills/rule/SKILL.md", over() <> "\n---\nshort tail\n"}],
        exit: 1,
        output: [".claude/skills/rule/SKILL.md: #{@limit + 1 + 16}"]
      },
      %{
        name:
          "fails on a frontmatter that never closes, rather than reading it as all frontmatter",
        files: [{".claude/skills/open/SKILL.md", "---\nname: x\n" <> over()}],
        exit: 1,
        output: [".claude/skills/open/SKILL.md: its frontmatter has no closing ---"]
      },
      %{
        name: "reads a skill under a non-ASCII directory and the ones after it",
        files: [
          {".claude/skills/é/SKILL.md", skill("a")},
          {".claude/skills/zz/SKILL.md", skill(over())}
        ],
        exit: 1,
        output: over_limit(".claude/skills/zz/SKILL.md")
      },
      %{
        name: "reports a skill past the limit under a non-ASCII directory",
        files: [{".claude/skills/é/SKILL.md", skill(over())}],
        exit: 1,
        output: over_limit(".claude/skills/é/SKILL.md")
      },
      %{
        name: "passes a short skill under a non-ASCII directory",
        files: [{".claude/skills/é/SKILL.md", skill("a")}],
        exit: 0,
        output: ""
      },
      # The check reads the working tree, so a tracked skill whose file is gone from
      # disk with the deletion unstaged must not end the run before the others are read.
      %{
        name: "passes over a tracked skill deleted from disk and reads the rest",
        files: [
          {".claude/skills/aa/SKILL.md", skill("a")},
          {".claude/skills/zz/SKILL.md", skill(over())}
        ],
        delete: [".claude/skills/aa/SKILL.md"],
        exit: 1,
        output: over_limit(".claude/skills/zz/SKILL.md")
      }
    ]
    |> Enum.map(&Map.put(&1, :task, @task))
  end
end

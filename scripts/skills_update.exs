#!/usr/bin/env elixir

defmodule SkillsUpdate do
  @usage """
  Usage: skills-update [package...]
         skills-update --check

  Updates APM skills in a linked worktree, then shows what changed and fails on
  anything in them that would run without a person reading it first. Nothing has
  taken effect at that point: ~/.claude/skills is the main checkout, and it gets
  the new copies only from an `apm install` there after the merge.

    --check     only look at the skills apm.lock.yaml deploys here
    --help, -h  print this message

  No check reads what a skill's text tells the agent to do, or what its scripts
  do. That part is the diff, which the pull request does not show, because
  .gitignore holds the skills out of the repository.
  """

  @skills_dir ".claude/skills"

  # What Claude Code or APM acts on by itself: a plugin manifest makes the folder a
  # plugin loaded every session, and APM integrates the other three unless
  # allowExecutables in apm.yml holds them back.
  @active_paths [".claude-plugin", "hooks", "bin", ".mcp.json"]

  def main(argv) do
    case argv do
      ["--check"] -> System.halt(if findings() == [], do: 0, else: 1)
      [flag] when flag in ["--help", "-h"] -> IO.puts(@usage)
      ["-" <> _ | _] -> die(@usage, 2)
      packages -> update(packages)
    end
  end

  defp update(packages) do
    unless linked_worktree?() do
      die("Run this in a linked worktree: ~/.claude/skills is this checkout's copy.", 1)
    end

    # A new worktree holds only the skills git tracks, and the diff needs the pins.
    apm!(["install"])
    pins = pins()

    work = Path.join(System.tmp_dir!(), "skills-update-#{System.unique_integer([:positive])}")

    try do
      snapshot(Path.join(work, "before"))
      apm!(["update", "--yes" | packages])
      # `apm update` can leave deployed files removed, which only this restores.
      apm!(["install"])
      snapshot(Path.join(work, "after"))

      report_ages(pins)
      diff(work)
      audited = match?({_, 0}, apm(["audit"]))
      System.halt(if audited and findings() == [], do: 0, else: 1)
    after
      File.rm_rf!(work)
    end
  end

  defp linked_worktree? do
    git!(["rev-parse", "--path-format=absolute", "--git-dir"]) !=
      git!(["rev-parse", "--path-format=absolute", "--git-common-dir"])
  end

  defp snapshot(to) do
    File.mkdir_p!(to)
    Enum.each(roots(), &File.cp_r!(&1, Path.join(to, Path.basename(&1))))
  end

  defp report_ages(before) do
    changed = Enum.reject(lock(), &(before[&1["name"]] == &1["resolved_commit"]))

    if changed == [] do
      IO.puts("No skill moved to a new revision.")
    else
      IO.puts("\nLast change upstream, by the commit's own date, which its author sets:")
      Enum.each(changed, &IO.puts("  #{&1["name"]}: #{age(&1)}"))
      IO.puts("")
    end
  end

  defp age(dep) do
    {args, field} =
      if dep["host"] == "gist.github.com" do
        id = dep["repo_url"] |> String.split("/") |> List.last()
        {"gists/#{id}/commits?per_page=1", ".[0].committed_at"}
      else
        query = if dep["virtual_path"], do: "path=#{dep["virtual_path"]}&", else: ""
        {"repos/#{dep["repo_url"]}/commits?#{query}per_page=1", ".[0].commit.committer.date"}
      end

    with {out, 0} <- System.cmd("gh", ["api", args, "--jq", field]),
         {:ok, date, _} <- DateTime.from_iso8601(String.trim(out)) do
      "#{div(DateTime.diff(DateTime.utc_now(), date), 86_400)}d ago"
    else
      _ -> "?"
    end
  end

  defp diff(work) do
    args = ["diff", "--no-index", "--no-ext-diff", "before", "after"]

    # `--no-index` exits 1 on a difference, as it means to here.
    case System.cmd("git", args, cd: work, into: IO.stream()) do
      {_, code} when code in [0, 1] -> :ok
      {_, code} -> die("git diff failed (exit #{code})", code)
    end
  end

  defp findings do
    found = Enum.sort(outside_skills() ++ Enum.flat_map(roots(), &in_skill/1))
    Enum.each(found, &IO.puts(:stderr, "[fail] #{&1}"))
    found
  end

  defp outside_skills do
    for path <- deployed(), not String.starts_with?(path, @skills_dir <> "/") do
      "#{path}: deployed outside #{@skills_dir}"
    end
  end

  defp in_skill(root) do
    files = Path.wildcard(root <> "/**", match_dot: true)

    active =
      for path <- files,
          Enum.any?(Path.split(Path.relative_to(path, root)), &(&1 in @active_paths)),
          File.regular?(path) do
        "#{path}: runs without being invoked"
      end

    skills = Enum.filter(files, &(Path.basename(&1) == "SKILL.md"))

    active ++ Enum.flat_map(skills, &in_skill_md/1)
  end

  defp in_skill_md(path) do
    text = File.read!(path)

    frontmatter =
      case Regex.run(~r/\A---\r?\n(.*?)^---/ms, text) do
        [_, block] -> block
        nil -> ""
      end

    keys =
      for [_, key] <- Regex.scan(~r/^["']?(hooks|allowed-tools)["']?\s*:/m, frontmatter) do
        "#{path}: #{key} in the frontmatter"
      end

    # An inline `!` runs only at the start of a line or after whitespace, so
    # `KEY=!` followed by a command stays literal, as Claude Code documents.
    commands =
      if text =~ ~r/(^|\s)!`/m or text =~ ~r/^ {0,3}(`{3,}|~{3,})\s*!/m,
        do: ["#{path}: a command run on invocation"],
        else: []

    keys ++ commands
  end

  defp roots do
    deployed()
    |> Enum.filter(&String.starts_with?(&1, @skills_dir <> "/"))
    |> Enum.map(&(&1 |> Path.split() |> Enum.take(3) |> Path.join()))
    |> Enum.uniq()
  end

  defp deployed, do: Enum.flat_map(lock(), &Map.get(&1, "deployed_files", []))

  defp pins, do: Map.new(lock(), &{&1["name"], &1["resolved_commit"]})

  defp lock do
    case System.cmd("yq", ["-o=json", "apm.lock.yaml"]) do
      {out, 0} -> JSON.decode!(out)["dependencies"]
      {_, code} -> die("yq could not read apm.lock.yaml (exit #{code})", code)
    end
  end

  defp apm(args), do: System.cmd("apm", args, into: IO.stream(), stderr_to_stdout: true)

  defp apm!(args) do
    case apm(args) do
      {_, 0} -> :ok
      {_, code} -> die("apm #{Enum.join(args, " ")} failed (exit #{code})", code)
    end
  end

  defp git!(args) do
    case System.cmd("git", args) do
      {out, 0} -> String.trim(out)
      {_, code} -> die("git #{Enum.join(args, " ")} failed (exit #{code})", code)
    end
  end

  defp die(message, code) do
    IO.puts(:stderr, message)
    System.halt(code)
  end
end

SkillsUpdate.main(System.argv())

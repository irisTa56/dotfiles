# Checks that the shared tasks under tasks/ do what they promise, by running each
# one's script on cases: a repository the case sets up, the arguments it passes,
# and the exit status and output it expects. The tasks are shell, so the projects
# that take them need none of this one; this project provides the check, and its
# own tests test this module rather than any task.
defmodule Dotfiles.TaskCases do
  @case_modules [Dotfiles.TaskCases.SkillsSizeCheck]

  # `root` is the repository whose tasks/ the cases run.
  def main([], root), do: run(Enum.flat_map(@case_modules, & &1.cases()), root)

  # Each case is a map: `name`; `task`, the script's path under `root`; `files`, as
  # {path, content}, written into a fresh git repository and staged; `delete`, paths
  # removed from disk after staging; `repository: false` for a directory git does
  # not track; `args`; `exit`; and `output`, as for output_mismatches/2.
  def run(cases, root) do
    failures = for c <- cases, failure = failure(c, root), do: failure

    if failures != [] do
      Enum.each(failures, &IO.puts(:stderr, &1))
      exit({:shutdown, 1})
    end
  end

  defp failure(c, root) do
    dir =
      Path.join(System.tmp_dir!(), "task-case-#{Base.url_encode64(:crypto.strong_rand_bytes(9))}")

    File.mkdir!(dir)

    try do
      check(c, root, dir)
    after
      File.rm_rf!(dir)
    end
  end

  defp check(c, root, dir) do
    # A git hook running this sets GIT_DIR and the like for its own repository,
    # which would turn every git call here toward that one; the ceiling keeps a
    # case with no repository of its own from finding one above it.
    env =
      for({name, _} <- System.get_env(), String.starts_with?(name, "GIT_"), do: {name, nil}) ++
        [{"GIT_CEILING_DIRECTORIES", Path.dirname(dir)}]

    opts = [cd: dir, env: env, stderr_to_stdout: true]
    repository? = Map.get(c, :repository, true)

    if repository?, do: git!(["init", "-q"], opts)

    for {path, content} <- c.files do
      file = Path.join(dir, path)
      File.mkdir_p!(Path.dirname(file))
      File.write!(file, content)
    end

    if repository?, do: git!(["add", "-A"], opts)
    Enum.each(Map.get(c, :delete, []), &File.rm!(Path.join(dir, &1)))

    {out, code} =
      Dotfiles.Command.cmd("bash", [Path.join(root, c.task) | Map.get(c, :args, [])], opts)

    case mismatches(c, code, out) do
      [] -> nil
      found -> "#{c.task}, #{c.name}: #{Enum.join(found, "; ")}; output:\n#{out}"
    end
  end

  # Setting up the case's repository is this check's own doing, so a failure there
  # stops the run rather than reading as the task's.
  defp git!(args, opts) do
    case Dotfiles.Command.cmd("git", args, opts) do
      {_, 0} -> :ok
      {out, code} -> raise "git #{Enum.join(args, " ")} exited #{code}: #{out}"
    end
  end

  defp mismatches(c, code, out) do
    exit = if code != c.exit, do: ["expected exit #{c.exit}, got #{code}"], else: []
    exit ++ output_mismatches(c.output, out)
  end

  # `expected` is either the whole output, "" for a run that must print nothing,
  # or a list of parts the output must contain.
  defp output_mismatches(whole, out) when is_binary(whole),
    do: if(out == whole, do: [], else: ["expected output #{inspect(whole)}"])

  defp output_mismatches(parts, out),
    do: for(part <- parts, not String.contains?(out, part), do: "lacks #{inspect(part)}")
end

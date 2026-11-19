# Checks that the shared tasks under tasks/ do what they promise, by running each
# one's script on cases: a repository the case sets up, the arguments it passes,
# and the exit status and output it expects. The tasks are shell, some wrapping a
# Rust binary, so the projects that take them need none of this one; this project
# provides the check, and its own tests test this module rather than any task.
defmodule Dotfiles.TaskCases do
  @case_modules [Dotfiles.TaskCases.SkillsSizeCheck, Dotfiles.TaskCases.UpstreamPinCheck]

  # `root` is the repository whose tasks/ the cases run.
  def main([], root), do: run(Enum.flat_map(@case_modules, & &1.cases()), root)

  # Each case is a map: `name`; `task`, the script's path under `root`; `files`, as
  # {path, content}, written into a fresh git repository and staged; `delete`, paths
  # removed from disk after staging; `repository: false` for a directory git does
  # not track; `git`, git commands run in the repository after staging, as lists of
  # arguments; `args`; `env`, as {name, value}, for the task's run; `exit`; `output`,
  # as for output_mismatches/2; `verify`, a function given the repository after the
  # run that returns what it found amiss; and `prepare`, as for failure/2.
  def run(cases, root) do
    failures = for c <- cases, failure = failure(c, root), do: failure

    if failures != [] do
      Enum.each(failures, &IO.puts(:stderr, &1))
      exit({:shutdown, 1})
    end
  end

  # `prepare`, where a case has one, is given a directory of its own outside the
  # case's repository, for the repositories the task reads beside that one, and
  # returns fields that are merged into the case before it runs.
  defp failure(c, root) do
    dir = scratch_dir()
    aux = scratch_dir()
    shim = scratch_dir()

    try do
      c = if prepare = c[:prepare], do: Map.merge(c, prepare.(aux)), else: c
      check(c, root, dir, shim)
    after
      File.rm_rf!(dir)
      File.rm_rf!(aux)
      File.rm_rf!(shim)
    end
  end

  # A task wrapping the dotfiles-tasks binary installs the release it pins with
  # `mise install <tool>` and runs it with `mise x <tool> -- <command>`; here a stand-in
  # for that mise installs nothing, exiting as TASK_CASE_MISE_INSTALL_STATUS says, and
  # runs the command alone, which finds the build of this working tree that
  # `rust:build` leaves under rust/target.
  @mise_shim """
  #!/bin/sh
  [ "$1" = install ] && exit "${TASK_CASE_MISE_INSTALL_STATUS:-0}"
  while [ "$#" -gt 0 ] && [ "$1" != -- ]; do shift; done
  shift
  exec "$@"
  """

  defp path(root, shim) do
    mise = Path.join(shim, "mise")
    File.write!(mise, @mise_shim)
    File.chmod!(mise, 0o755)
    build = Path.join(root, "rust/target/debug")
    {"PATH", Enum.join([shim, build | List.wrap(System.get_env("PATH"))], ":")}
  end

  defp scratch_dir do
    dir =
      Path.join(System.tmp_dir!(), "task-case-#{Base.url_encode64(:crypto.strong_rand_bytes(9))}")

    File.mkdir!(dir)
    dir
  end

  @doc """
  The environment entries that unset every `GIT_` variable, for a git run that must
  act on the repository it is given. A git hook running this exports `GIT_DIR` and
  its kin for its own repository, which would turn every git call toward that one.
  """
  def cleared_git_env,
    do: for({name, _} <- System.get_env(), String.starts_with?(name, "GIT_"), do: {name, nil})

  defp check(c, root, dir, shim) do
    # The ceiling keeps a case with no repository of its own from finding one above it.
    env = cleared_git_env() ++ [{"GIT_CEILING_DIRECTORIES", Path.dirname(dir)}]

    opts = [cd: dir, env: env, stderr_to_stdout: true]
    repository? = Map.get(c, :repository, true)

    if repository?, do: git!(["init", "-q"], opts)

    for {path, content} <- c.files do
      file = Path.join(dir, path)
      File.mkdir_p!(Path.dirname(file))
      File.write!(file, content)
    end

    if repository?, do: git!(["add", "-A"], opts)
    Enum.each(Map.get(c, :git, []), &git!(&1, opts))
    Enum.each(Map.get(c, :delete, []), &File.rm!(Path.join(dir, &1)))

    path = path(root, shim)

    {out, code} =
      Dotfiles.Command.cmd(
        "bash",
        [Path.join(root, c.task) | Map.get(c, :args, [])],
        Keyword.update!(opts, :env, &(&1 ++ [path | Map.get(c, :env, [])]))
      )

    verified = if verify = c[:verify], do: verify.(dir), else: []

    case mismatches(c, code, out) ++ verified do
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

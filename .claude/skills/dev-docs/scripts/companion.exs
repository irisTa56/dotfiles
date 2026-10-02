# Sets this clone up for its companion repository, or checks that it is set up.
#
#   elixir companion.exs check
#   elixir companion.exs setup <url> <directory>
#
# Everything is written to the clone's git config, which linked worktrees share.

defmodule Companion do
  @key "dev-docs.private-workspace"
  @remote "checkpoints"
  @refspec "refs/entire/checkpoints/*:refs/entire/checkpoints/*"
  # Git appends the hook's arguments to the command, which the trailing `#` drops.
  # A clone that holds no checkpoint yet has nothing to push, and pushing nothing fails.
  @hook ~S[test "$1" = checkpoints || test -z "$(git for-each-ref --count=1 refs/entire/checkpoints)" || git push --quiet checkpoints #]

  def main(["check"]), do: report(missing())

  def main(["setup", url, directory]) do
    directory = Path.expand(directory)

    with :ok <- outside_repository(directory),
         :ok <- clone(url, directory),
         :ok <- remote(url) do
      git!(["config", @key, directory])

      if uses_entire?() do
        git!(["config", "remote.#{@remote}.push", @refspec])
        git!(["config", "hook.checkpoints-sync.event", "pre-push"])
        git!(["config", "hook.checkpoints-sync.command", @hook])
      end

      report(missing())
    else
      {:refused, why} ->
        IO.puts(:stderr, "refused: #{why}")
        exit({:shutdown, 1})
    end
  end

  def main(_) do
    IO.puts(:stderr, "usage: companion.exs check | setup <url> <directory>")
    exit({:shutdown, 2})
  end

  defp missing do
    workspace =
      case git(["config", "--get", "--type=path", @key]) do
        {:ok, ""} -> ["git config #{@key}"]
        {:ok, path} -> if File.dir?(path), do: [], else: ["the directory #{path}"]
        :error -> ["git config #{@key}"]
      end

    entire =
      if uses_entire?() do
        [
          {git(["remote", "get-url", @remote]) != :error, "the git remote #{@remote}"},
          {git(["config", "--get", "remote.#{@remote}.push"]) == {:ok, @refspec},
           "git config remote.#{@remote}.push"},
          {git(["config", "--get", "hook.checkpoints-sync.command"]) == {:ok, @hook},
           "git config hook.checkpoints-sync"}
        ]
        |> Enum.reject(&elem(&1, 0))
        |> Enum.map(&elem(&1, 1))
      else
        []
      end

    workspace ++ entire
  end

  defp report([]), do: :ok

  defp report(missing) do
    Enum.each(missing, &IO.puts("missing: #{&1}"))
    exit({:shutdown, 1})
  end

  defp outside_repository(directory) do
    {:ok, common} = git(["rev-parse", "--path-format=absolute", "--git-common-dir"])
    main = Path.dirname(common)

    if String.starts_with?(directory <> "/", main <> "/"),
      do: {:refused, "#{directory} is inside the repository at #{main}"},
      else: :ok
  end

  defp clone(url, directory) do
    if File.dir?(directory) do
      :ok
    else
      File.mkdir_p!(Path.dirname(directory))
      git!(["clone", "--quiet", url, directory])
      :ok
    end
  end

  defp remote(url) do
    case {uses_entire?(), git(["remote", "get-url", @remote])} do
      {false, _} -> :ok
      {true, {:ok, ^url}} -> :ok
      {true, {:ok, other}} -> {:refused, "the git remote #{@remote} already points at #{other}"}
      {true, :error} -> git!(["remote", "add", @remote, url]) && :ok
    end
  end

  defp uses_entire? do
    {:ok, top} = git(["rev-parse", "--show-toplevel"])
    File.dir?(Path.join(top, ".entire"))
  end

  defp git(args) do
    case System.cmd("git", args, stderr_to_stdout: true) do
      {out, 0} -> {:ok, String.trim(out)}
      _ -> :error
    end
  end

  defp git!(args) do
    case System.cmd("git", args, stderr_to_stdout: true) do
      {out, 0} -> String.trim(out)
      {out, _} -> raise "git #{Enum.join(args, " ")} failed: #{out}"
    end
  end
end

Companion.main(System.argv())

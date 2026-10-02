# Sets this clone up for its companion repository, or checks that it is set up.
#
#   elixir companion.exs check
#   elixir companion.exs setup <url> <directory>
#
# Everything is written to the clone's git config, which linked worktrees share.
# What is guarded is what would otherwise go wrong without a sign; a mistake that
# fails by itself is left to fail.

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

    with :ok <- placement(directory),
         :ok <- remote(url),
         :ok <- clone(url, directory) do
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
        {:ok, path} when path != "" ->
          if checkout?(path), do: [], else: ["a git checkout at #{path}"]

        _ ->
          ["git config #{@key}"]
      end

    entire =
      if uses_entire?() do
        [
          {push_remote() == @remote,
           "checkpoint_push_remote set to #{@remote} in .entire/settings.json"},
          {git(["remote", "get-url", @remote]) != :error, "the git remote #{@remote}"},
          {git(["config", "--get", "remote.#{@remote}.push"]) == {:ok, @refspec},
           "git config remote.#{@remote}.push"},
          {git(["config", "--get", "hook.checkpoints-sync.command"]) == {:ok, @hook} and
             hook_listed?(), "the pre-push hook checkpoints-sync (git 2.54 or later runs it)"}
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

  # Inside a working tree the workspace would be untracked content there, which
  # removing a worktree deletes and another repository's commits can pick up.
  defp placement(directory) do
    parent = existing(Path.dirname(directory))

    cond do
      git(["rev-parse", "--is-inside-work-tree"], parent) == {:ok, "true"} ->
        {:refused, "#{directory} is inside a git working tree"}

      File.exists?(directory) and not checkout?(directory) ->
        {:refused, "#{directory} exists and is not the top of a git checkout"}

      true ->
        :ok
    end
  end

  defp existing(path), do: if(File.dir?(path), do: path, else: existing(Path.dirname(path)))

  defp checkout?(path),
    do: File.dir?(path) and git(["rev-parse", "--show-prefix"], path) == {:ok, ""}

  defp clone(url, directory) do
    unless File.dir?(directory) do
      File.mkdir_p!(Path.dirname(directory))
      git!(["clone", "--quiet", url, directory])
    end

    :ok
  end

  defp remote(url) do
    case {uses_entire?(), git(["config", "--get", "remote.#{@remote}.url"])} do
      {false, _} -> :ok
      {true, {:ok, ^url}} -> :ok
      {true, {:ok, other}} -> {:refused, "the git remote #{@remote} already points at #{other}"}
      {true, :error} -> git!(["remote", "add", @remote, url]) && :ok
    end
  end

  # Entire sends checkpoints to `origin` unless its settings name another remote,
  # and a local settings file overrides the committed one.
  defp push_remote do
    Enum.find_value(["settings.local.json", "settings.json"], fn file ->
      with {:ok, text} <- File.read(Path.join([top(), ".entire", file])),
           {:ok, %{"strategy_options" => %{"checkpoint_push_remote" => name}}} <-
             JSON.decode(text) do
        name
      else
        _ -> nil
      end
    end)
  end

  defp hook_listed? do
    case git(["hook", "list", "pre-push"]) do
      {:ok, out} -> "checkpoints-sync" in String.split(out)
      :error -> false
    end
  end

  defp uses_entire?, do: File.dir?(Path.join(top(), ".entire"))

  defp top do
    {:ok, top} = git(["rev-parse", "--show-toplevel"])
    top
  end

  defp git(args, dir \\ nil) do
    case System.cmd("git", args, [stderr_to_stdout: true] ++ if(dir, do: [cd: dir], else: [])) do
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

defmodule Dotfiles.SkillsUpdate do
  @usage """
  Usage: mise run skills:update [package...]

  Updates APM skills, every one or those named as `apm update` names them, holding
  back any whose last change upstream merged less than a day ago, as the release
  windows set for npm, uv and Hex do. APM has no such window of its own.

  The time comes from the pull request that merged the skill's last commit into the
  default branch, which GitHub records, since a commit's own date is whatever its
  author set. A commit GitHub lists no such pull request for is held too, though the
  list can miss one, so that hold is for checking by hand. `apm update <package>`
  followed by `apm install` is the way past a hold.

    --help, -h  print this message
  """

  @window_seconds 86_400

  # `root` is the directory holding apm.yml and apm.lock.yaml, where apm runs.
  def main(argv, root) do
    case argv do
      [flag] when flag in ["--help", "-h"] -> IO.puts(@usage)
      ["-" <> _ | _] -> die(@usage, 2)
      packages -> update(packages, root)
    end
  end

  defp update(packages, root) do
    {ready, held} =
      packages
      |> chosen(root)
      |> Enum.map(&with_hold/1)
      |> Enum.split_with(&(&1.hold == nil))

    Enum.each(held, &IO.puts("#{&1.dep["name"]}: held, #{&1.hold}"))

    if ready == [] do
      IO.puts("Nothing to update.")
    else
      apm!(["update", "--yes" | Enum.map(ready, &key(&1.dep))], root)
      # `apm update` can leave deployed files removed, which only this restores.
      apm!(["install"], root)
      apm!(["audit"], root)
    end
  end

  defp chosen(packages, root) do
    deps = lock(root)

    case packages -- Enum.map(deps, &key/1) do
      [] -> Enum.filter(deps, &(packages == [] or key(&1) in packages))
      unknown -> die("Not in apm.lock.yaml: #{Enum.join(unknown, " ")}", 1)
    end
  end

  # What `apm update` takes: the path into the repository, or a gist's alias.
  defp key(%{"virtual_path" => path, "repo_url" => repo}) when is_binary(path),
    do: "#{repo}/#{path}"

  defp key(dep), do: dep["name"]

  defp with_hold(dep), do: %{dep: dep, hold: hold(dep)}

  # A gist is the owner's own, and `skills:push` is what moves it.
  defp hold(%{"host" => "gist.github.com"}), do: nil

  # GitHub can list only the pull request that took a commit into another branch, whose
  # merge time says nothing of when the commit reached the default one.
  @merged_into_default "[.[] | select(.base.ref == .base.repo.default_branch) | .merged_at | select(.)] | max // empty"

  defp hold(dep) do
    query = if dep["virtual_path"], do: "path=#{dep["virtual_path"]}&", else: ""

    with {:ok, sha} <-
           gh(
             "repos/#{dep["repo_url"]}/commits?#{query}per_page=1",
             ".[0].sha // empty",
             "no commit on this path"
           ),
         {:ok, merged} <-
           gh(
             "repos/#{dep["repo_url"]}/commits/#{sha}/pulls",
             @merged_into_default,
             "GitHub lists no merge into the default branch for its last commit; check by hand"
           ),
         {:ok, time, _} <- DateTime.from_iso8601(merged) do
      if DateTime.diff(DateTime.utc_now(), time) >= @window_seconds,
        do: nil,
        else: "last change merged #{merged}"
    else
      {:error, reason} -> reason
    end
  end

  defp gh(path, jq, when_empty) do
    case System.cmd("gh", ["api", path, "--jq", jq]) do
      {out, 0} ->
        case String.trim(out) do
          "" -> {:error, when_empty}
          value -> {:ok, value}
        end

      {_, code} ->
        {:error, "GitHub request failed (exit #{code})"}
    end
  end

  defp lock(root) do
    case System.cmd("yq", ["-o=json", "apm.lock.yaml"], cd: root) do
      {out, 0} -> JSON.decode!(out)["dependencies"]
      {_, code} -> die("yq could not read apm.lock.yaml (exit #{code})", code)
    end
  end

  defp apm!(args, root) do
    case System.cmd("apm", args, cd: root, into: IO.stream(), stderr_to_stdout: true) do
      {_, 0} -> :ok
      {_, code} -> die("apm #{Enum.join(args, " ")} failed (exit #{code})", code)
    end
  end

  # `exit({:shutdown, code})` rather than halting, so elixir's own CLI ends the
  # run with that status once its output is flushed, and a test can catch it.
  defp die(message, code) do
    IO.puts(:stderr, message)
    exit({:shutdown, code})
  end
end

defmodule Mix.Tasks.Dotfiles.SkillsUpdate do
  @shortdoc "Updates APM skills, holding back any changed upstream under a day ago"
  @moduledoc @shortdoc
  use Mix.Task

  @requirements ["compile"]

  # The repository this project sits in, whatever directory the task runs from.
  @impl true
  def run(argv),
    do:
      Dotfiles.SkillsUpdate.main(
        argv,
        Mix.Project.project_file() |> Path.dirname() |> Path.dirname()
      )
end

#!/usr/bin/env elixir

defmodule SkillsUpdate do
  @usage """
  Usage: skills-update [package...]

  Updates APM skills, every one or those named as `apm update` names them, holding
  back any whose last change upstream merged less than a day ago, as the release
  windows set for npm, uv and Hex do. APM has no such window of its own.

  The time comes from the pull request that merged the skill's last commit, which
  GitHub records, since a commit's own date is whatever its author set. A commit no
  merged pull request holds is held too, and `apm update` is the way past that.

    --help, -h  print this message
  """

  @window_seconds 86_400

  def main(argv) do
    case argv do
      [flag] when flag in ["--help", "-h"] -> IO.puts(@usage)
      ["-" <> _ | _] -> die(@usage, 2)
      packages -> update(packages)
    end
  end

  defp update(packages) do
    {ready, held} =
      packages |> chosen() |> Enum.map(&with_hold/1) |> Enum.split_with(&(&1.hold == nil))

    Enum.each(held, &IO.puts("#{&1.dep["name"]}: held, #{&1.hold}"))

    if ready == [] do
      IO.puts("Nothing to update.")
    else
      apm!(["update", "--yes" | Enum.map(ready, &key(&1.dep))])
      # `apm update` can leave deployed files removed, which only this restores.
      apm!(["install"])
      apm!(["audit"])
    end
  end

  defp chosen(packages) do
    deps = lock()

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

  defp hold(dep) do
    query = if dep["virtual_path"], do: "path=#{dep["virtual_path"]}&", else: ""

    with {:ok, sha} <-
           gh("repos/#{dep["repo_url"]}/commits?#{query}per_page=1", ".[0].sha // empty"),
         {:ok, merged} <-
           gh(
             "repos/#{dep["repo_url"]}/commits/#{sha}/pulls",
             "[.[].merged_at | select(.)] | max // empty"
           ),
         {:ok, time, _} <- DateTime.from_iso8601(merged) do
      if DateTime.diff(DateTime.utc_now(), time) >= @window_seconds,
        do: nil,
        else: "last change merged #{merged}"
    else
      {:error, reason} -> reason
      _ -> "no readable merge time"
    end
  end

  defp gh(path, jq) do
    case System.cmd("gh", ["api", path, "--jq", jq]) do
      {out, 0} ->
        case String.trim(out) do
          "" -> {:error, "no merged pull request found"}
          value -> {:ok, value}
        end

      {_, code} ->
        {:error, "GitHub request failed (exit #{code})"}
    end
  end

  defp lock do
    case System.cmd("yq", ["-o=json", "apm.lock.yaml"]) do
      {out, 0} -> JSON.decode!(out)["dependencies"]
      {_, code} -> die("yq could not read apm.lock.yaml (exit #{code})", code)
    end
  end

  defp apm!(args) do
    case System.cmd("apm", args, into: IO.stream(), stderr_to_stdout: true) do
      {_, 0} -> :ok
      {_, code} -> die("apm #{Enum.join(args, " ")} failed (exit #{code})", code)
    end
  end

  defp die(message, code) do
    IO.puts(:stderr, message)
    System.halt(code)
  end
end

SkillsUpdate.main(System.argv())

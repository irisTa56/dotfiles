defmodule Mix.Tasks.Dotfiles.ReleasePins do
  @shortdoc "Fails where a shared task pins another dotfiles-tasks release than rust/Cargo.toml's"
  @moduledoc @shortdoc
  use Mix.Task

  @requirements ["compile"]

  # The repository this project sits in, whatever directory the task runs from.
  @impl true
  def run([]) do
    root = Mix.Project.project_file() |> Path.dirname() |> Path.dirname()

    case Dotfiles.ReleasePins.mismatches(root) do
      [] ->
        :ok

      found ->
        Enum.each(found, &IO.puts(:stderr, &1))
        exit({:shutdown, 1})
    end
  end
end

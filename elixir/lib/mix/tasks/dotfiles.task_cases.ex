defmodule Mix.Tasks.Dotfiles.TaskCases do
  @shortdoc "Fails where a shared task under tasks/ does not do what its cases expect"
  @moduledoc @shortdoc
  use Mix.Task

  @requirements ["compile"]

  # The repository this project sits in, whatever directory the task runs from.
  @impl true
  def run(argv),
    do:
      Dotfiles.TaskCases.main(
        argv,
        Mix.Project.project_file() |> Path.dirname() |> Path.dirname()
      )
end

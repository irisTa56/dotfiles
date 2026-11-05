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

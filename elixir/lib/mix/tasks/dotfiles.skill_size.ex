defmodule Mix.Tasks.Dotfiles.SkillSize do
  @shortdoc "Fails where a tracked skill's body is past what a compaction re-attaches"
  @moduledoc @shortdoc
  use Mix.Task

  @requirements ["compile"]

  # The repository this project sits in, whatever directory the task runs from.
  @impl true
  def run(argv),
    do:
      Dotfiles.SkillSize.main(
        argv,
        Mix.Project.project_file() |> Path.dirname() |> Path.dirname()
      )
end

defmodule Mix.Tasks.Dotfiles.WithNewTerminal do
  @shortdoc "Runs a shell script in a new Terminal window"
  @moduledoc @shortdoc
  use Mix.Task

  @requirements ["compile"]

  @impl true
  def run(argv), do: Dotfiles.WithNewTerminal.main(argv)
end

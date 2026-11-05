defmodule Mix.Tasks.Dotfiles.CcusagePopup do
  @shortdoc "Shows Claude Code usage and cost in a Quick Look popup"
  @moduledoc @shortdoc
  use Mix.Task

  @requirements ["compile"]

  @impl true
  def run(argv), do: Dotfiles.CcusagePopup.main(argv)
end

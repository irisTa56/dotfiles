defmodule Mix.Tasks.Dotfiles.MiseMultiUpgrade do
  @shortdoc "Upgrades mise tools directory by directory, asking for each first"
  @moduledoc @shortdoc
  use Mix.Task

  @requirements ["compile"]

  # A non-zero status leaves through `exit({:shutdown, status})`, which elixir's
  # own CLI turns into the exit status on the same way out as a task that ends
  # normally, rather than halting the VM from here.
  @impl true
  def run(argv) do
    case Dotfiles.MiseMultiUpgrade.main(argv) do
      0 -> :ok
      status -> exit({:shutdown, status})
    end
  end
end

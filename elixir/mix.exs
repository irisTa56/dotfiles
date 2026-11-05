# The Elixir commands this repository runs through mise tasks and shell
# functions, each a Mix task run from this directory.
defmodule DotfilesScripts.MixProject do
  use Mix.Project

  def project do
    [
      app: :dotfiles_scripts,
      version: "0.1.0",
      elixir: "~> 1.20",
      deps: []
    ]
  end

  def application do
    [extra_applications: [:crypto]]
  end
end

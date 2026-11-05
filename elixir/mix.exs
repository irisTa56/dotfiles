# The Elixir commands of this repository, each a Mix task in the `dotfiles`
# namespace that a mise task or a shell function runs from this directory.
defmodule Dotfiles.MixProject do
  use Mix.Project

  def project do
    [
      app: :dotfiles,
      version: "0.1.0",
      elixir: "~> 1.20",
      start_permanent: Mix.env() == :prod,
      elixirc_paths: elixirc_paths(Mix.env()),
      deps: deps()
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      # :crypto for the random name of the ccusage popup's temporary directory.
      extra_applications: [:logger, :crypto]
    ]
  end

  # The fakes the tests stand in for external commands with, compiled for the
  # test environment alone.
  defp elixirc_paths(:test), do: ["lib", "test/support"]
  defp elixirc_paths(_), do: ["lib"]

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      # Test only: qa:ex runs it with MIX_ENV=test, and the tasks this project
      # exists for run in dev, where a dep would have to be fetched, and fetched
      # again after each version bump, before any of them could start.
      {:credo, "~> 1.7", only: :test, runtime: false}
    ]
  end
end

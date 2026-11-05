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

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:credo, "~> 1.7", only: [:dev, :test], runtime: false}
    ]
  end
end

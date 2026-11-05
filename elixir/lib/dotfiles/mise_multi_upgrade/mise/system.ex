defmodule Dotfiles.MiseMultiUpgrade.Mise.System do
  @behaviour Dotfiles.MiseMultiUpgrade.Mise
  @impl true
  def cmd(args, opts), do: System.cmd("mise", args, opts)
end

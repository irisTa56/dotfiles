defmodule Dotfiles.MiseMultiUpgrade.Mise.System do
  @moduledoc false
  @behaviour Dotfiles.MiseMultiUpgrade.Mise
  @impl true
  def cmd(args, opts), do: System.cmd("mise", args, opts)
end

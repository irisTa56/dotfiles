defmodule Dotfiles.Command.System do
  @behaviour Dotfiles.Command
  @impl true
  def cmd(command, args, opts), do: System.cmd(command, args, opts)
end

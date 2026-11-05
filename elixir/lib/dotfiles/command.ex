# Every external command a Dotfiles module runs goes through `cmd/3`, which hands
# it to the implementation the :command key of the :dotfiles application names,
# so a unit test can answer in each command's place without starting a process.
defmodule Dotfiles.Command do
  @callback cmd(command :: String.t(), args :: [String.t()], opts :: keyword()) ::
              {Collectable.t(), non_neg_integer()}

  def cmd(command, args, opts \\ []),
    do: Application.get_env(:dotfiles, :command, Dotfiles.Command.System).cmd(command, args, opts)
end

# Changes to the environment a test makes, each undone once the test exits.
defmodule Dotfiles.TestEnv do
  # Makes every external command a real one for the rest of the test, as an
  # integration test does.
  def real_commands do
    Application.put_env(:dotfiles, :command, Dotfiles.Command.System)

    ExUnit.Callbacks.on_exit(fn ->
      Application.put_env(:dotfiles, :command, Dotfiles.FakeCommand)
    end)

    :ok
  end

  # Sets environment variables for the rest of the test, as {name, value} pairs,
  # where a nil value unsets one.
  def put_env(env) do
    saved = for {name, _} <- env, do: {name, System.get_env(name)}
    ExUnit.Callbacks.on_exit(fn -> restore(saved) end)
    restore(env)
  end

  defp restore(env) do
    for {name, value} <- env,
        do: if(value, do: System.put_env(name, value), else: System.delete_env(name))

    :ok
  end
end

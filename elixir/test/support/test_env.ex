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

  # A directory of the test's own under the system's temporary one, removed when
  # the test exits; every test that needs one takes it from here. Not ExUnit's
  # :tmp_dir, which sits in this project: in an agent's worktree its path holds
  # .claude/worktrees/, which CcusagePopup counts toward the checkout above, and
  # mise there loads the configs of the repository around it. The name is
  # random, since System.unique_integer/1 is unique within one VM only: two
  # suites running at once drew the same name, and one's cleanup removed the
  # other's directory mid-run. File.mkdir!/1 fails on a name already taken
  # rather than sharing it.
  def tmp_dir!(prefix) do
    dir =
      Path.join(System.tmp_dir!(), "#{prefix}-#{Base.url_encode64(:crypto.strong_rand_bytes(9))}")

    File.mkdir!(dir)
    ExUnit.Callbacks.on_exit(fn -> File.rm_rf!(dir) end)
    dir
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

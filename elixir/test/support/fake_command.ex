# Answers in place of every external command, through the handler a test installs,
# and logs each call. A module under test may run commands from tasks of its own,
# so the handler and the log live in an Agent rather than in the test's process.
defmodule Dotfiles.FakeCommand do
  @behaviour Dotfiles.Command

  # `handler` takes a command, its arguments and its options, and returns what
  # `System.cmd/3` would; a call it has no clause for fails the test. Installed
  # again within a test, it replaces the handler and keeps the log.
  def install(handler) do
    if Process.whereis(__MODULE__) do
      Agent.update(__MODULE__, &%{&1 | handler: handler})
    else
      ExUnit.Callbacks.start_supervised!(%{
        id: __MODULE__,
        start:
          {Agent, :start_link, [fn -> %{handler: handler, calls: []} end, [name: __MODULE__]]}
      })
    end

    :ok
  end

  # Every call so far, oldest first, as {command, args, opts}.
  def calls, do: Agent.get(__MODULE__, &Enum.reverse(&1.calls))

  # The arguments of every call to `command` so far, oldest first.
  def calls(command), do: for({^command, args, _} <- calls(), do: args)

  # The handler runs in the caller's process, so it may itself read the log or
  # the files it is handed.
  @impl true
  def cmd(command, args, opts) do
    handler =
      Agent.get_and_update(__MODULE__, fn state ->
        {state.handler, %{state | calls: [{command, args, opts} | state.calls]}}
      end)

    handler.(command, args, opts)
  end
end

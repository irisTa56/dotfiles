# Starts the task as mix starts it, in a process of its own: only then does the
# status the task exits with become the exit status of the command.
defmodule Dotfiles.Integration.MiseMultiUpgradeTaskTest do
  use ExUnit.Case
  @moduletag :integration

  # In the test environment, whose build this suite has already compiled.
  defp mix(arg) do
    System.cmd("mix", ["dotfiles.mise_multi_upgrade", arg],
      cd: Path.dirname(Mix.Project.project_file()),
      env: [{"MIX_ENV", "test"}],
      stderr_to_stdout: true
    )
  end

  test "exits 2 on an unknown flag and 0 on -h" do
    {out, 2} = mix("--bogus")
    assert out =~ "Usage: mise run mise:multi-upgrade"

    {out, 0} = mix("-h")
    assert out =~ "Usage: mise run mise:multi-upgrade"
  end
end

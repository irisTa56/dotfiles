# Runs the runner with real git and bash, against a script of the test's own.
defmodule Dotfiles.Integration.TaskCasesTest do
  use ExUnit.Case
  @moduletag :integration
  import ExUnit.CaptureIO

  setup do
    Dotfiles.TestEnv.real_commands()
    root = Dotfiles.TestEnv.tmp_dir!("task-cases-test")
    # The script lists what git tracks and what is on disk, then exits 3.
    File.mkdir_p!(Path.join(root, "tasks/x"))

    File.write!(Path.join(root, "tasks/x/y.sh"), """
    git ls-files
    ls s
    exit 3
    """)

    {:ok, root: root}
  end

  test "runs the task in a repository holding the case's files", %{root: root} do
    c = %{
      name: "lists",
      task: "tasks/x/y.sh",
      files: [{"s/kept.md", "k"}, {"s/gone.md", "g"}],
      delete: ["s/gone.md"],
      exit: 3,
      output: "s/gone.md\ns/kept.md\nkept.md\n"
    }

    assert capture_io(:stderr, fn -> Dotfiles.TaskCases.run([c], root) end) == ""
  end
end

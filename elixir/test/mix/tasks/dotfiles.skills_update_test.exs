defmodule Mix.Tasks.Dotfiles.SkillsUpdateTest do
  use ExUnit.Case
  import ExUnit.CaptureIO

  alias Dotfiles.FakeCommand
  alias Mix.Tasks.Dotfiles.SkillsUpdate

  # The task runs from the Mix project, so the root it hands on is the
  # repository above it, which holds apm.yml.
  test "reads the lockfile of the repository the project sits in" do
    root = Mix.Project.project_file() |> Path.dirname() |> Path.dirname()
    FakeCommand.install(fn "yq", _, _ -> {~s({"dependencies": []}), 0} end)

    assert capture_io(fn -> SkillsUpdate.run([]) end) =~ "Nothing to update."
    assert [{"yq", _, opts}] = FakeCommand.calls()
    assert opts[:cd] == root
    assert File.exists?(Path.join(root, "apm.yml"))
  end
end

defmodule Mix.Tasks.Dotfiles.SkillsUpdateTest do
  use ExUnit.Case
  import ExUnit.CaptureIO

  # The task runs from the Mix project, so the root it hands on is the
  # repository above it, which holds apm.yml.
  test "reads the lockfile of the repository the project sits in" do
    root = Mix.Project.project_file() |> Path.dirname() |> Path.dirname()
    Dotfiles.FakeCommand.install(fn "yq", _, _ -> {~s({"dependencies": []}), 0} end)

    assert capture_io(fn -> Mix.Tasks.Dotfiles.SkillsUpdate.run([]) end) =~ "Nothing to update."
    assert [{"yq", _, opts}] = Dotfiles.FakeCommand.calls()
    assert opts[:cd] == root
    assert File.exists?(Path.join(root, "apm.yml"))
  end
end

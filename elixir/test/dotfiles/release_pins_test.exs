defmodule Dotfiles.ReleasePinsTest do
  use ExUnit.Case

  setup do
    {:ok, root: Dotfiles.TestEnv.tmp_dir!("release-pins-test")}
  end

  defp put(root, path, content) do
    file = Path.join(root, path)
    File.mkdir_p!(Path.dirname(file))
    File.write!(file, content)
  end

  defp cargo(root, version),
    do:
      put(
        root,
        "rust/Cargo.toml",
        ~s([package]\nname = "dotfiles-tasks"\nversion = "#{version}"\n)
      )

  defp wrapper(root, path, version),
    do:
      put(
        root,
        path,
        ~s(#!/usr/bin/env bash\n#MISE tools={"github:o/r"={version="#{version}", bin="x"}} # x-release-please-version\nexec x\n)
      )

  test "names a wrapper that pins another version than rust/Cargo.toml's", %{root: root} do
    cargo(root, "0.2.0")
    wrapper(root, "tasks/a/b.sh", "0.1.0")

    assert Dotfiles.ReleasePins.mismatches(root) == [
             "tasks/a/b.sh pins 0.1.0, but rust/Cargo.toml is at 0.2.0"
           ]
  end

  # A version line that lost its marker, or a wrapper gone from tasks/, would leave
  # nothing to compare, and the check would pass having checked nothing.
  test "reports where no shared task carries the release-please marker", %{root: root} do
    cargo(root, "0.2.0")
    put(root, "tasks/a/b.sh", "#!/usr/bin/env bash\necho hi\n")

    assert Dotfiles.ReleasePins.mismatches(root) == [
             "no file under tasks/ carries x-release-please-version, so no wrapper pins a release"
           ]
  end

  # tasks/README.md names the marker in its prose; a task is a *.sh file.
  test "reads the marker only from shell tasks", %{root: root} do
    cargo(root, "0.2.0")
    wrapper(root, "tasks/a/b.sh", "0.2.0")
    put(root, "tasks/README.md", "Its version line ends in `# x-release-please-version`.\n")

    assert Dotfiles.ReleasePins.mismatches(root) == []
  end
end

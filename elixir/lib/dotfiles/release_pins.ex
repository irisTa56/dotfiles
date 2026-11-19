# Checks that each shared task wrapping the dotfiles-tasks binary pins the version
# rust/Cargo.toml is at. The release pull request moves both together, and
# release-please skips a file its config names but cannot find without saying so, so
# a wrapper moved or renamed would go on pinning an old release unnoticed.
defmodule Dotfiles.ReleasePins do
  @marker "x-release-please-version"

  # What is amiss under `root`, one line each; [] where nothing is.
  def mismatches(root) do
    case wrappers(root) do
      [] ->
        ["no file under tasks/ carries #{@marker}, so no wrapper pins a release"]

      paths ->
        version = cargo_version(root)

        Enum.flat_map(paths, &mismatch(&1, File.read!(Path.join(root, &1)), version))
    end
  end

  defp wrappers(root) do
    root
    |> Path.join("tasks/**/*.sh")
    |> Path.wildcard()
    |> Enum.filter(&(File.regular?(&1) and File.read!(&1) =~ @marker))
    |> Enum.map(&Path.relative_to(&1, root))
    |> Enum.sort()
  end

  defp mismatch(path, text, version) do
    case pinned_version(text) do
      ^version -> []
      nil -> ["#{path} carries #{@marker} on a line with no @<version>"]
      pinned -> ["#{path} pins #{pinned}, but rust/Cargo.toml is at #{version}"]
    end
  end

  # The version follows the `@` of the tool spec on the marked line; nil where it has none.
  defp pinned_version(text) do
    case Regex.run(~r/@([0-9][^'"\s]*).*#{@marker}/, text) do
      [_, version] -> version
      nil -> nil
    end
  end

  defp cargo_version(root) do
    [_, version] =
      Regex.run(~r/^version = "([^"]+)"/m, File.read!(Path.join(root, "rust/Cargo.toml")))

    version
  end
end

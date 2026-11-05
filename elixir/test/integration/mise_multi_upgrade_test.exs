# Runs the module against the real git and the real mise. mise works from a
# throwaway home of its own, holding a local asdf plugin, `dummy`, that lists
# versions 1.0.0, 1.1.0 and 2.0.0 and installs any of them without the network.
defmodule Dotfiles.Integration.MiseMultiUpgradeTest do
  use ExUnit.Case
  @moduletag :integration
  import ExUnit.CaptureIO

  # Everything lives in a directory of its own under the system's temporary one,
  # outside this repository, so no config of the repository is an ancestor of
  # any directory mise runs in; mise is also stopped from looking above it, and
  # the code's mise calls that name no directory run from it. The run takes in
  # temporary directories, since on Linux the system's is /tmp, which the code
  # otherwise leaves out.
  setup do
    Dotfiles.TestEnv.real_commands()

    base =
      Path.join(System.tmp_dir!(), "mise-multi-upgrade-#{System.unique_integer([:positive])}")

    File.mkdir_p!(base)
    on_exit(fn -> File.rm_rf!(base) end)
    {dir, 0} = System.cmd("pwd", ["-P"], cd: base)
    dir = String.trim(dir)
    mise = Path.join(dir, "mise")

    # A git hook running this suite sets GIT_DIR and the like for its own
    # repository, which would turn every git call here toward that one; a shell
    # mise is activated in sets __MISE_SESSION and the like, from which mise runs
    # the hooks of the configs that shell has loaded, this repository's among them.
    inherited =
      for {name, _} <- System.get_env(),
          String.starts_with?(name, ["GIT_", "MISE_", "__MISE_"]),
          do: {name, nil}

    Dotfiles.TestEnv.put_env(
      inherited ++
        [
          {"GIT_CEILING_DIRECTORIES", dir},
          {"MISE_DATA_DIR", Path.join(mise, "data")},
          {"MISE_CONFIG_DIR", Path.join(mise, "config")},
          {"MISE_STATE_DIR", Path.join(mise, "state")},
          {"MISE_CACHE_DIR", Path.join(mise, "cache")},
          {"MISE_GLOBAL_CONFIG_FILE", Path.join(mise, "config/config.toml")},
          {"MISE_SYSTEM_CONFIG_FILE", Path.join(mise, "system.toml")},
          {"MISE_TRUSTED_CONFIG_PATHS", dir},
          {"MISE_CEILING_PATHS", dir},
          {"MISE_NO_HOOKS", "1"}
        ]
    )

    cwd = File.cwd!()
    File.cd!(dir)
    on_exit(fn -> File.cd!(cwd) end)

    plugin(Path.join(mise, "plugin"))
    cmd!("mise", ["plugins", "link", "dummy", Path.join(mise, "plugin")], dir)
    {:ok, dir: dir}
  end

  defp plugin(dir) do
    scripts = %{
      "list-all" => ~s(echo "1.0.0 1.1.0 2.0.0"),
      "download" => "exit 0",
      "install" => """
      mkdir -p "$ASDF_INSTALL_PATH/bin"
      printf '#!/bin/sh\\necho %s\\n' "$ASDF_INSTALL_VERSION" > "$ASDF_INSTALL_PATH/bin/dummy"
      chmod +x "$ASDF_INSTALL_PATH/bin/dummy"
      """
    }

    for {name, body} <- scripts do
      path = Path.join([dir, "bin", name])
      File.mkdir_p!(Path.dirname(path))
      File.write!(path, "#!/bin/sh\n" <> body)
      File.chmod!(path, 0o755)
    end
  end

  defp cmd!(command, args, cwd) do
    {out, code} = System.cmd(command, args, cd: cwd, stderr_to_stdout: true)
    assert code == 0, "#{command} #{Enum.join(args, " ")}: #{out}"
    out
  end

  # A git repository at `dir` whose committed mise.toml requests dummy 1, with
  # 1.0.0 installed, and which mise tracks.
  defp project(dir) do
    File.mkdir_p!(dir)
    File.write!(Path.join(dir, "mise.toml"), ~s([tools]\ndummy = "1"\n))
    cmd!("git", ["init", "-q"], dir)
    cmd!("git", ["add", "."], dir)
    cmd!("git", ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "init"], dir)
    cmd!("mise", ["install", "dummy@1.0.0"], dir)
    cmd!("mise", ["ls"], dir)
    dir
  end

  defp current(dir), do: dir |> then(&cmd!("mise", ["current", "dummy"], &1)) |> String.trim()

  # Fails before the module runs if mise tracks any config outside the test's
  # directory, which a run that answers yes would upgrade, should the isolation
  # above ever stop holding.
  defp run(args, answers) do
    tracked = cmd!("mise", ["config", "ls", "--tracked-configs"], File.cwd!())

    for path <- String.split(tracked, "\n", trim: true),
        do: assert(String.starts_with?(path, File.cwd!() <> "/"), path)

    {{code, out}, err} =
      with_io(:stderr, fn -> with_io(answers, fn -> Dotfiles.MiseMultiUpgrade.main(args) end) end)

    {out <> err, code}
  end

  test "shows a project's table from mise and upgrades it within its request", %{dir: dir} do
    project = project(Path.join(dir, "project"))

    {out, 0} = run(["--include-tmpdirs", "--dry-run"], "")

    assert out =~ ~r/^dummy\s+1\s+1\.0\.0\s+-\s+1\.1\.0\s+-\s+2\.0\.0\s+-$/m

    {out, 0} = run(["--include-tmpdirs"], "y\n")

    assert out =~ "==> mise -C #{project} upgrade --no-global"
    assert current(project) == "1.1.0"
    assert File.read!(Path.join(project, "mise.toml")) =~ ~s(dummy = "1")
  end

  test "leaves out a linked worktree that git reports", %{dir: dir} do
    project = project(Path.join(dir, "project"))
    worktree = Path.join(project, ".claude/worktrees/wt")
    cmd!("git", ["worktree", "add", "-q", worktree], project)
    cmd!("mise", ["ls"], worktree)
    assert cmd!("mise", ["config", "ls", "--tracked-configs"], dir) =~ worktree

    {out, 0} = run(["--include-tmpdirs", "--dry-run"], "")
    refute out =~ worktree

    {out, 0} = run(["--include-tmpdirs", "--dry-run", "--include-worktrees"], "")
    assert out =~ worktree
  end
end

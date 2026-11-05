defmodule Dotfiles.WithNewTerminalTest do
  use ExUnit.Case
  import ExUnit.CaptureIO

  alias Dotfiles.FakeCommand

  @moduletag :tmp_dir

  setup %{tmp_dir: tmp} do
    File.write!(Path.join(tmp, "job.sh"), """
    #!/bin/zsh
    # a comment

      # an indented comment
    echo "a\\b" && ls
    """)

    :ok
  end

  # The exit status, stderr, and the AppleScript osascript was handed, which
  # answers with `status`.
  defp run(argv, status \\ 0) do
    FakeCommand.install(fn "osascript", ["-e", _], _ -> {"", status} end)

    err =
      capture_io(:stderr, fn ->
        send(self(), {:code, code(fn -> Dotfiles.WithNewTerminal.main(argv) end)})
      end)

    assert_received {:code, code}
    {code, err, Enum.map(FakeCommand.calls("osascript"), &List.last/1)}
  end

  defp code(fun) do
    fun.()
    0
  catch
    :exit, {:shutdown, code} -> code
  end

  test "runs the script in a new window, without its comment and blank lines", %{tmp_dir: tmp} do
    {0, _, [script]} = run([Path.join(tmp, "job.sh")])

    assert script == ~S"""
           tell application "Terminal"
             do script "setopt no_bang_hist && clear\necho \"a\\b\" && ls"
             set custom title of front window to "job.sh"
           end tell
           """
  end

  test "changes to --cwd first and takes the title from --title", %{tmp_dir: tmp} do
    {0, _, [script]} = run([Path.join(tmp, "job.sh"), "--cwd", "/srv/app", "--title", ~s(a "b")])

    assert script =~ ~s(do script "setopt no_bang_hist && clear\\ncd /srv/app\\necho)
    assert script =~ ~s(set custom title of front window to "a \\"b\\"")
  end

  test "reads a relative script path from --from", %{tmp_dir: tmp} do
    {0, _, [script]} = run(["job.sh", "--from", tmp])

    assert script =~ "&& ls"
  end

  test "fails with osascript's status when it fails", %{tmp_dir: tmp} do
    {1, err, _} = run([Path.join(tmp, "job.sh")], 1)

    assert err =~ "osascript failed (exit 1)"
  end

  test "refuses anything but one script path with its usage and exit 1" do
    for argv <- [[], ["a.sh", "b.sh"], ["a.sh", "--bogus"]] do
      {1, err, []} = run(argv)
      assert err =~ "Usage: with-new-terminal"
    end
  end
end

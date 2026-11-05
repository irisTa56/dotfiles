defmodule Mix.Tasks.Dotfiles.MiseMultiUpgradeTest do
  use ExUnit.Case
  import ExUnit.CaptureIO

  alias Mix.Tasks.Dotfiles.MiseMultiUpgrade

  test "leaves with the status main returns when it is not zero" do
    capture_io(:stderr, fn ->
      assert catch_exit(MiseMultiUpgrade.run(["--bogus"])) == {:shutdown, 2}
    end)
  end

  test "returns normally when main returns zero" do
    capture_io(fn -> assert MiseMultiUpgrade.run(["-h"]) == :ok end)
  end
end

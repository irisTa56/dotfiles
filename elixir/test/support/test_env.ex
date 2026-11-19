# What a test sets up outside itself, each undone once the test exits.
defmodule Dotfiles.TestEnv do
  # A directory of the test's own under the system's temporary one, removed when
  # the test exits; every test that needs one takes it from here. The name is
  # random, since System.unique_integer/1 is unique within one VM only: two suites
  # running at once drew the same name, and one's cleanup removed the other's
  # directory mid-run. File.mkdir!/1 fails on a name already taken rather than
  # sharing it.
  def tmp_dir!(prefix) do
    dir =
      Path.join(System.tmp_dir!(), "#{prefix}-#{Base.url_encode64(:crypto.strong_rand_bytes(9))}")

    File.mkdir!(dir)
    ExUnit.Callbacks.on_exit(fn -> File.rm_rf!(dir) end)
    dir
  end
end

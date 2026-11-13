# The pre-commit hook runs this suite, and git hands the hook GIT_DIR and the
# like for its own repository; this pins that none of them reaches a test.
defmodule Dotfiles.InheritedEnvTest do
  use ExUnit.Case, async: true

  test "no GIT_ variable from the environment the suite started in reaches a test" do
    assert for({name, _} <- System.get_env(), String.starts_with?(name, "GIT_"), do: name) == []
  end
end

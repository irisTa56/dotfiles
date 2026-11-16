defmodule Dotfiles.TaskCasesTest do
  use ExUnit.Case
  import ExUnit.CaptureIO

  alias Dotfiles.FakeCommand

  # A case for the script tasks/x/y.sh that expects `exit` and `output`.
  defp a_case(fields),
    do: Map.merge(%{name: "a case", task: "tasks/x/y.sh", files: [], exit: 0, output: ""}, fields)

  # Answers git as a fresh repository does, and the task with `ran`.
  defp fake(ran) do
    FakeCommand.install(fn
      "git", _, _ -> {"", 0}
      "bash", _, opts -> ran.(opts)
    end)
  end

  # The exit status, as elixir's CLI would set it, and what was printed.
  defp run(cases) do
    {{code, out}, err} =
      with_io(:stderr, fn -> with_io(fn -> status(cases) end) end)

    {out <> err, code}
  end

  defp status(cases) do
    Dotfiles.TaskCases.run(cases, "/repo")
    0
  catch
    :exit, {:shutdown, code} -> code
  end

  test "passes a case whose task exits and prints as expected" do
    fake(fn _ -> {"too long\n", 1} end)

    assert run([a_case(%{exit: 1, output: ["too long"]})]) == {"", 0}
  end

  # A git hook running the check sets GIT_DIR for its own repository, which would
  # turn the case's git calls toward that one.
  test "runs the task in a repository of the case's own, its files staged and the deleted ones gone" do
    Dotfiles.TestEnv.put_env([{"GIT_DIR", "/elsewhere/.git"}])
    parent = self()

    FakeCommand.install(fn
      "git", args, opts ->
        send(parent, {:git, args, opts})
        {"", 0}

      "bash", args, opts ->
        dir = opts[:cd]
        send(parent, {:bash, args, dir, opts[:env], File.ls!(Path.join(dir, "s"))})
        {"", 0}
    end)

    assert run([
             a_case(%{
               files: [{"s/kept.md", "k"}, {"s/gone.md", "g"}],
               delete: ["s/gone.md"],
               args: ["s/*"]
             })
           ]) == {"", 0}

    assert_received {:git, ["init", "-q"], init_opts}
    assert_received {:git, ["add", "-A"], add_opts}
    assert_received {:bash, ["/repo/tasks/x/y.sh", "s/*"], dir, env, ["kept.md"]}
    assert init_opts[:cd] == dir and add_opts[:cd] == dir
    assert {"GIT_DIR", nil} in env and {"GIT_DIR", nil} in init_opts[:env]
    refute File.exists?(dir)
  end

  test "runs a case marked repository: false in a directory git does not track" do
    parent = self()

    FakeCommand.install(fn
      "git", args, _ ->
        flunk("git #{inspect(args)} ran")

      "bash", _, opts ->
        send(parent, {:bash, opts[:cd]})
        {"", 0}
    end)

    assert run([a_case(%{repository: false})]) == {"", 0}
    assert_received {:bash, dir} when is_binary(dir)
  end

  test "runs a TypeScript task with node rather than bash" do
    parent = self()

    FakeCommand.install(fn
      "git", _, _ ->
        {"", 0}

      "node", args, _ ->
        send(parent, {:node, args})
        {"", 0}
    end)

    assert run([a_case(%{task: "tasks/x/y.ts", args: ["a"]})]) == {"", 0}
    assert_received {:node, ["/repo/tasks/x/y.ts", "a"]}
  end

  # A case whose task reads another repository, an upstream, sets that one up apart
  # from the case's own, and names it in the files and arguments it then gives.
  test "runs prepare in a directory apart from the case's repository and takes the fields it returns" do
    parent = self()

    FakeCommand.install(fn
      "git", _, _ ->
        {"", 0}

      "bash", args, opts ->
        send(parent, {:bash, args, opts[:cd], File.read!(Path.join(opts[:cd], "f"))})
        {"", 0}
    end)

    prepare = fn aux ->
      send(parent, {:aux, aux, File.dir?(aux)})
      %{files: [{"f", aux}], args: [aux]}
    end

    assert run([a_case(%{prepare: prepare})]) == {"", 0}
    assert_received {:aux, aux, true}
    assert_received {:bash, ["/repo/tasks/x/y.sh", ^aux], dir, ^aux}
    refute String.starts_with?(aux, dir) or String.starts_with?(dir, aux)
    refute File.exists?(aux)
  end

  test "runs the task with the variables a case's env sets" do
    parent = self()

    FakeCommand.install(fn
      "git", _, _ ->
        {"", 0}

      "bash", _, opts ->
        send(parent, {:env, opts[:env]})
        {"", 0}
    end)

    assert run([a_case(%{env: [{"GITHUB_ACTIONS", "true"}]})]) == {"", 0}
    assert_received {:env, env}
    assert {"GITHUB_ACTIONS", "true"} in env
  end

  # A task that pushes is set up with a remote, which only exists once the case's
  # repository does.
  test "runs a case's git commands in its repository after staging its files" do
    parent = self()

    FakeCommand.install(fn
      "git", args, opts ->
        send(parent, {:git, args, opts[:cd]})
        {"", 0}

      "bash", _, _ ->
        {"", 0}
    end)

    assert run([a_case(%{git: [["commit", "-qm", "base"]]})]) == {"", 0}
    assert_received {:git, ["add", "-A"], dir}
    assert_received {:git, ["commit", "-qm", "base"], ^dir}
  end

  test "reports what a case's verify finds in the repository after the task ran" do
    parent = self()
    fake(fn _ -> {"", 0} end)

    verify = fn dir ->
      send(parent, {:verify, File.dir?(dir)})
      ["mise.toml still holds the old pin"]
    end

    {out, code} = run([a_case(%{verify: verify})])

    assert code == 1
    assert_received {:verify, true}
    assert out =~ "tasks/x/y.sh, a case: mise.toml still holds the old pin"
  end

  test "reports every case whose task exits otherwise, and fails" do
    fake(fn _ -> {"", 0} end)

    {out, code} =
      run([a_case(%{name: "first", exit: 1}), a_case(%{name: "second", exit: 2})])

    assert code == 1
    assert out =~ "tasks/x/y.sh, first: expected exit 1, got 0"
    assert out =~ "tasks/x/y.sh, second: expected exit 2, got 0"
  end

  test "reports a case whose output lacks an expected part" do
    fake(fn _ -> {"something else\n", 1} end)

    {out, code} = run([a_case(%{exit: 1, output: ["too long", "19800"]})])

    assert code == 1
    assert out =~ "lacks \"too long\""
    assert out =~ "something else"
  end

  test "reports a case expecting no output that got some" do
    fake(fn _ -> {"noise\n", 0} end)

    {out, code} = run([a_case(%{exit: 0, output: ""})])

    assert code == 1
    assert out =~ "expected output \"\""
    assert out =~ "noise"
  end
end

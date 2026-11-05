defmodule Dotfiles.CcusagePopupTest do
  use ExUnit.Case
  import ExUnit.CaptureIO

  alias Dotfiles.FakeCommand

  # Session logs live under a directory that CLAUDE_CONFIG_DIR names, and each
  # recorded cwd that should exist is a directory there; git answers for those
  # directories from `commons`, as {directory, the common dir it reports}.
  setup do
    tmp = Dotfiles.TestEnv.tmp_dir!("ccusage-popup-test")
    File.mkdir!(Path.join(tmp, "projects"))
    Dotfiles.TestEnv.put_env([{"CLAUDE_CONFIG_DIR", tmp}])
    {:ok, tmp_dir: tmp}
  end

  # Records `cwd` as the cwd of `project`'s session log.
  defp session(tmp, project, cwd) do
    dir = Path.join([tmp, "projects", project])
    File.mkdir_p!(dir)
    File.write!(Path.join(dir, "s.jsonl"), ~s({"type":"summary"}\n{"cwd":#{JSON.encode!(cwd)}}\n))
  end

  defp day(date, cost, opts \\ []) do
    %{
      "date" => date,
      "inputTokens" => Keyword.get(opts, :input, 1000),
      "outputTokens" => 10,
      "cacheCreationTokens" => Keyword.get(opts, :create, 0),
      "cacheReadTokens" => Keyword.get(opts, :read, 0),
      "totalTokens" => 1010,
      "totalCost" => cost,
      "modelBreakdowns" => [
        %{"modelName" => Keyword.get(opts, :model, "claude-opus-4-20250514"), "cost" => cost}
      ]
    }
  end

  # Runs the popup with ccusage answering `usage` (a map to encode, or
  # {output, status}) and git answering from `commons`. Returns the exit status,
  # stderr, and the HTML Quick Look was handed, or nil when it was not reached.
  defp run(argv, usage, commons \\ %{}, quick_look \\ 0) do
    test = self()

    FakeCommand.install(fn
      "ccusage", _, _ ->
        if is_map(usage), do: {JSON.encode!(usage), 0}, else: usage

      "git", ["-C", dir, "rev-parse", "--path-format=absolute", "--git-common-dir"], _ ->
        case commons[dir] do
          nil -> {"fatal: not a git repository\n", 128}
          common -> {common <> "\n", 0}
        end

      "qlmanage", ["-p", html], _ ->
        send(test, {:html, html, File.read!(html)})
        {"", quick_look}
    end)

    err =
      capture_io(:stderr, fn ->
        send(test, {:status, status(fn -> Dotfiles.CcusagePopup.main(argv) end)})
      end)

    assert_received {:status, code}

    html =
      receive do
        {:html, path, html} ->
          refute File.exists?(Path.dirname(path))
          html
      after
        0 -> nil
      end

    {code, err, html}
  end

  defp status(fun) do
    fun.()
    0
  catch
    :exit, {:shutdown, code} -> code
  end

  defp today do
    {date, _} = :calendar.local_time()
    Date.from_erl!(date)
  end

  test "asks ccusage for the last two weeks of daily usage per project" do
    run(["daily"], %{"totals" => %{"totalTokens" => 0}})

    since = today() |> Date.add(-14) |> Calendar.strftime("%Y%m%d")

    assert FakeCommand.calls("ccusage") == [
             ["claude", "daily", "--instances", "--json", "--since", since]
           ]
  end

  test "asks for nine weeks from a Sunday, and twelve months from the first" do
    run(["weekly"], %{"totals" => %{"totalTokens" => 0}})
    run(["monthly"], %{"totals" => %{"totalTokens" => 0}})

    [weekly, monthly] =
      for args <- FakeCommand.calls("ccusage") do
        <<y::binary-4, m::binary-2, d::binary-2>> = List.last(args)
        Date.from_iso8601!("#{y}-#{m}-#{d}")
      end

    assert Date.day_of_week(weekly, :sunday) == 1
    assert Date.diff(today(), weekly) in 56..62
    assert monthly == today() |> Date.beginning_of_month() |> Date.shift(month: -11)
  end

  test "counts each project toward the repository its sessions ran in", %{tmp_dir: tmp} do
    alpha = Path.join(tmp, "repos/alpha")
    beta = Path.join(tmp, "repos/beta")
    plain = Path.join(tmp, "plain")
    for dir <- [alpha, beta, plain], do: File.mkdir_p!(dir)

    session(tmp, "-alpha", alpha)
    session(tmp, "-alpha-wt", Path.join(alpha, ".claude/worktrees/x/sub"))
    session(tmp, "-scratch", "/private/tmp/claude-501/-alpha/abc/scratchpad")
    session(tmp, "-beta", beta)
    session(tmp, "-moved", "/nowhere/moved-repo")
    session(tmp, "-gone", "/tmp/gone")
    session(tmp, "-plain", plain)

    usage = %{
      "projects" => %{
        "-alpha" => [day("2026-11-01", 1.0)],
        "-alpha-wt" => [day("2026-11-01", 2.0)],
        "-scratch" => [day("2026-11-01", 4.0)],
        "-beta" => [day("2026-11-01", 8.0)],
        "-moved" => [day("2026-11-01", 16.0)],
        "-gone" => [day("2026-11-01", 0.25)],
        "-plain" => [day("2026-11-01", 0.5)],
        "-nolog" => [day("2026-11-01", 32.0)]
      }
    }

    {0, _, html} =
      run(["daily"], usage, %{alpha => Path.join(alpha, ".git"), beta => "/srv/git/beta.git"})

    [_, by_repo] = String.split(html, "Cost (USD) by repository - daily")

    assert by_repo =~
             ~r/^Date\s+moved-repo\s+beta\s+alpha\s+other\s+Total$/m

    assert by_repo =~ ~r/^2026-11-01\s+\$16\.00\s+\$8\.00\s+\$7\.00\s+\$32\.75\s+\$63\.75$/m
  end

  test "buckets days by week from Sunday and by month, and totals each table" do
    usage = %{
      "projects" => %{
        "-p" => [
          # A Saturday, then the Sunday that starts the next week.
          day("2026-10-31", 1.0, read: 3000, model: "claude-sonnet-4-5-20250929"),
          day("2026-11-01", 2.5)
        ]
      }
    }

    {0, _, weekly} = run(["weekly"], usage)
    {0, _, monthly} = run(["monthly"], usage)

    assert weekly =~ ~r/^Week\s+Input.*Cost \(USD\)$/m
    assert weekly =~ ~r/^2026-10-25\s+1,000\s+10\s+0\s+3,000\s+1,010\s+75\.0%\s+\$1\.00$/m
    assert weekly =~ ~r/^2026-11-01\s+1,000\s+10\s+0\s+0\s+1,010\s+0\.0%\s+\$2\.50$/m
    assert weekly =~ ~r/^Total\s+2,000\s+20\s+0\s+3,000\s+2,020\s+60\.0%\s+\$3\.50$/m
    assert weekly =~ ~r/^Week\s+opus-4\s+sonnet-4-5\s+Total$/m

    assert monthly =~ ~r/^2026-10\s+1,000/m
    assert monthly =~ ~r/^2026-11\s+1,000/m
  end

  test "escapes the report for HTML", %{tmp_dir: tmp} do
    dir = Path.join(tmp, "r&d")
    File.mkdir_p!(dir)
    session(tmp, "-rd", dir)

    {0, _, html} =
      run(["daily"], %{"projects" => %{"-rd" => [day("2026-11-01", 1.0, model: "<m>")]}}, %{
        dir => Path.join(dir, ".git")
      })

    assert html =~ "r&amp;d"
    assert html =~ "&lt;m&gt;"
    refute html =~ "<m>"
  end

  test "reads session logs from each CLAUDE_CONFIG_DIR entry that holds or is projects",
       %{tmp_dir: tmp} do
    holds = Path.join(tmp, "holds")
    named = Path.join(tmp, "other/projects")
    repo = Path.join(tmp, "repo")
    File.mkdir_p!(repo)
    session(holds, "-a", repo)

    File.mkdir_p!(Path.join(named, "-b"))
    File.write!(Path.join([named, "-b", "s.jsonl"]), ~s({"cwd":#{JSON.encode!(repo)}}\n))

    Dotfiles.TestEnv.put_env([{"CLAUDE_CONFIG_DIR", " #{holds} ,#{named},,#{tmp}/neither"}])

    usage = %{"projects" => %{"-a" => [day("2026-11-01", 1.0)], "-b" => [day("2026-11-01", 2.0)]}}
    {0, _, html} = run(["daily"], usage, %{repo => Path.join(repo, ".git")})

    assert html =~ ~r/^Date\s+repo\s+Total$/m
    refute html =~ "other"
  end

  test "says so when the range holds no usage" do
    {0, _, html} = run(["daily"], %{"totals" => %{"totalTokens" => 0}})

    assert html =~ "No usage data."
  end

  test "fails on ccusage output with no per-project breakdown" do
    {1, err, nil} = run(["daily"], %{"totals" => %{"totalTokens" => 5}})

    assert err =~ ~s[no per-project breakdown ("projects" key)]
  end

  test "fails with ccusage's status when ccusage fails" do
    {3, err, nil} = run(["daily"], {"", 3})

    assert err =~ "ccusage claude daily"
    assert err =~ "failed (exit 3)"
  end

  test "fails when Quick Look fails, removing the report all the same" do
    {1, err, html} = run(["daily"], %{"totals" => %{"totalTokens" => 0}}, %{}, 1)

    assert html =~ "No usage data."
    assert err =~ "Quick Look preview failed"
  end

  test "refuses another period with its usage and exit 2" do
    {2, err, nil} = run(["yearly"], %{})

    assert err =~ "Usage: mise run ccusage:<daily|weekly|monthly>"
    assert FakeCommand.calls() == []
  end
end

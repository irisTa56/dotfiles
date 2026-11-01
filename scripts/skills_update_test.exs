#!/usr/bin/env elixir

# A pre-commit hook sets these for the repository being committed to, and the fixtures'
# git commands would inherit them and act on that repository instead of on the fixture.
System.get_env()
|> Map.keys()
|> Enum.filter(&String.starts_with?(&1, "GIT_"))
|> Enum.each(&System.delete_env/1)

ExUnit.start()

defmodule SkillsUpdateCheckTest do
  use ExUnit.Case

  @script Path.expand("skills_update.exs", __DIR__)
  @skill "---\nname: probe\ndescription: d\n---\nbody\n"

  setup do
    dir = Path.join(System.tmp_dir!(), "skills-update-test-#{System.unique_integer([:positive])}")
    on_exit(fn -> File.rm_rf!(dir) end)
    File.mkdir_p!(dir)
    {:ok, dir: dir}
  end

  defp write(dir, path, content) do
    full = Path.join(dir, path)
    File.mkdir_p!(Path.dirname(full))
    File.write!(full, content)
  end

  # A lockfile deploying the skill, or `paths` where the test names its own.
  defp lock(dir, paths \\ [".claude/skills/probe", ".claude/skills/probe/SKILL.md"]) do
    listed = Enum.map_join(paths, "", &"  - #{&1}\n")

    write(
      dir,
      "apm.lock.yaml",
      "dependencies:\n- name: probe\n  resolved_commit: abc\n  deployed_files:\n#{listed}"
    )
  end

  # The fixture sits outside this repository, where mise's shim finds no yq version.
  # The fake apm directory comes first on PATH, and TMPDIR is the test's own.
  # A test that names its own PATH in `env` replaces the default one.
  defp run(dir, cwd, args, env \\ []) do
    {yq, 0} = System.cmd("mise", ["which", "yq"])
    tmp = Path.join(dir, "tmp")
    File.mkdir_p!(tmp)

    own = List.keyfind(env, "PATH", 0)
    env = List.keydelete(env, "PATH", 0)

    path =
      if own,
        do: elem(own, 1),
        else:
          Enum.join(
            [Path.join(dir, "bin"), Path.dirname(String.trim(yq)), System.get_env("PATH")],
            ":"
          )

    System.cmd("elixir", [@script | args],
      cd: cwd,
      env: [{"PATH", path}, {"TMPDIR", tmp} | env],
      stderr_to_stdout: true
    )
  end

  defp check(dir), do: run(dir, dir, ["--check"])

  defp assert_flagged(dir, text) do
    {out, code} = check(dir)
    assert code == 1, out
    assert out =~ text
  end

  defp assert_clean(dir) do
    {out, code} = check(dir)
    assert code == 0, out
  end

  test "a skill of plain instructions passes", %{dir: dir} do
    lock(dir)
    write(dir, ".claude/skills/probe/SKILL.md", @skill)
    write(dir, ".claude/skills/probe/scripts/run.py", "print()\n")

    assert_clean(dir)
  end

  for path <- [".claude-plugin/plugin.json", "hooks/hooks.json", ".mcp.json", "bin/tool"] do
    test "#{path} in a skill is flagged", %{dir: dir} do
      lock(dir)
      write(dir, ".claude/skills/probe/SKILL.md", @skill)
      write(dir, ".claude/skills/probe/#{unquote(path)}", "{}\n")

      assert_flagged(dir, ".claude/skills/probe/#{unquote(path)}")
    end
  end

  for key <- ["hooks", "allowed-tools"] do
    test "#{key} in the frontmatter is flagged", %{dir: dir} do
      lock(dir)

      write(
        dir,
        ".claude/skills/probe/SKILL.md",
        "---\nname: probe\n#{unquote(key)}: x\n---\nbody\n"
      )

      assert_flagged(dir, unquote(key))
    end
  end

  test "the same words in the body pass", %{dir: dir} do
    lock(dir)
    write(dir, ".claude/skills/probe/SKILL.md", @skill <> "hooks: none\nallowed-tools: none\n")

    assert_clean(dir)
  end

  for {name, body} <- [
        {"at the start of a line", "!`id`\n"},
        {"after whitespace", "- user: !`id`\n"},
        {"as a fence", "```!\nid\n```\n"}
      ] do
    test "a command #{name} is flagged", %{dir: dir} do
      lock(dir)
      write(dir, ".claude/skills/probe/SKILL.md", @skill <> unquote(body))

      assert_flagged(dir, "command")
    end
  end

  test "a placeholder that stays literal passes", %{dir: dir} do
    lock(dir)
    write(dir, ".claude/skills/probe/SKILL.md", @skill <> "KEY=!`id`\n")

    assert_clean(dir)
  end

  test "a file deployed outside the skills directory is flagged", %{dir: dir} do
    lock(dir, [".claude/rules/probe.md"])
    write(dir, ".claude/rules/probe.md", "rule\n")

    assert_flagged(dir, ".claude/rules/probe.md")
  end

  test "a skill the lockfile does not deploy is left alone", %{dir: dir} do
    lock(dir)
    write(dir, ".claude/skills/probe/SKILL.md", @skill)
    write(dir, ".claude/skills/global-hooks/.claude-plugin/plugin.json", "{}\n")

    assert_clean(dir)
  end

  test "a file name with control bytes is reported without the raw bytes", %{dir: dir} do
    lock(dir)
    write(dir, ".claude/skills/probe/SKILL.md", @skill)
    name = "ok-before\e[2K\rok-after.json"
    write(dir, ".claude/skills/probe/hooks/#{name}", "{}\n")

    assert File.exists?(Path.join(dir, ".claude/skills/probe/hooks/#{name}")),
           "fixture: the file was not written"

    {out, code} = check(dir)

    assert code == 1, out
    assert out =~ "ok-before", out
    assert out =~ "ok-after", out
    refute out =~ "\e", inspect(out)
    refute out =~ "\r", inspect(out)
  end

  test "a lockfile naming a skill directory that is missing is not a clean result", %{dir: dir} do
    lock(dir)

    {out, code} = check(dir)

    assert code == 1, "exit #{code}, output #{inspect(out)}"
    assert out =~ ".claude/skills/probe", out
  end

  # The filesystem here folds case, so these are the usual names under another spelling.
  defp assert_same_file(dir, written, usual) do
    assert File.exists?(Path.join(dir, usual)), "fixture: #{written} is not #{usual} here"
  end

  for {written, usual} <- [
        {".Claude-Plugin/plugin.json", ".claude-plugin/plugin.json"},
        {"Hooks/hooks.json", "hooks/hooks.json"}
      ] do
    test "#{written} is flagged as #{usual}", %{dir: dir} do
      lock(dir)
      write(dir, ".claude/skills/probe/SKILL.md", @skill)
      write(dir, ".claude/skills/probe/#{unquote(written)}", "{}\n")

      assert_same_file(dir, unquote(written), ".claude/skills/probe/#{unquote(usual)}")
      assert_flagged(dir, ".claude/skills/probe/#{unquote(written)}")
    end
  end

  for {name, content, text} <- [
        {"hooks in its frontmatter", "---\nname: probe\nhooks: x\n---\nbody\n", "hooks"},
        {"a command", @skill <> "!`id`\n", "command"}
      ] do
    test "skill.md with #{name} is flagged", %{dir: dir} do
      lock(dir)
      write(dir, ".claude/skills/probe/skill.md", unquote(content))

      assert_same_file(dir, "skill.md", ".claude/skills/probe/SKILL.md")
      assert_flagged(dir, unquote(text))
    end
  end

  test "an image line after a closed fence is not a command", %{dir: dir} do
    lock(dir)
    write(dir, ".claude/skills/probe/SKILL.md", @skill <> "```sh\nls\n```\n![img](x.png)\n")

    assert_clean(dir)
  end

  # The update path, run in a linked worktree with a fake apm.
  defp git(cwd, args) do
    base = ["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"]
    {_, 0} = System.cmd("git", base ++ args, cd: cwd, stderr_to_stdout: true)
  end

  defp repo(dir) do
    main = Path.join(dir, "main")
    File.mkdir_p!(main)
    git(main, ["init", "-q"])
    git(main, ["commit", "-q", "--allow-empty", "-m", "i"])
    main
  end

  # `install` does nothing, `update` appends $APM_APPEND to the deployed skill, and
  # `audit` exits $APM_AUDIT. Every call is logged to apm.log beside the repository.
  # With $APM_BUMP set, `update` also moves the lock's resolved_commit.
  defp fake_apm(dir) do
    write(dir, "bin/apm", """
    #!/bin/sh
    echo "$1" >> "#{dir}/apm.log"
    case "$1" in
      update)
        printf '%s\\n' "${APM_APPEND:-changed}" >> .claude/skills/probe/SKILL.md
        if [ -n "$APM_BUMP" ]; then
          sed 's/resolved_commit: abc/resolved_commit: def/' apm.lock.yaml > apm.lock.new
          mv apm.lock.new apm.lock.yaml
        fi ;;
      audit) exit "${APM_AUDIT:-0}" ;;
    esac
    """)

    File.chmod!(Path.join(dir, "bin/apm"), 0o755)
  end

  # Deploys the skill, plus a file outside the skills directory when `extra` is set,
  # which the check reports.
  defp worktree(dir, extra \\ []) do
    main = repo(dir)
    wt = Path.join(dir, "wt")
    git(main, ["worktree", "add", "-q", "-b", "wt", wt])
    fake_apm(dir)
    lock(wt, [".claude/skills/probe", ".claude/skills/probe/SKILL.md"] ++ extra)
    write(wt, ".claude/skills/probe/SKILL.md", @skill)
    wt
  end

  defp update(dir, wt, env), do: run(dir, wt, [], env)

  test "a failed audit still prints what the check reports", %{dir: dir} do
    wt = worktree(dir, [".claude/rules/probe.md"])
    write(wt, ".claude/rules/probe.md", "rule\n")

    {out, code} = update(dir, wt, [{"APM_AUDIT", "1"}])

    assert File.read!(Path.join(dir, "apm.log")) =~ "audit", "fixture: audit did not run"
    assert code != 0, out
    assert out =~ "[fail] .claude/rules/probe.md", out
  end

  for audit <- ["0", "1"] do
    test "a run with audit exiting #{audit} leaves nothing in TMPDIR", %{dir: dir} do
      wt = worktree(dir)

      {out, _} = update(dir, wt, [{"APM_AUDIT", unquote(audit)}])

      assert out =~ "+changed", "fixture: the update changed nothing\n" <> out
      assert File.ls!(Path.join(dir, "tmp")) == []
    end
  end

  test "control bytes in a changed line do not reach stdout raw", %{dir: dir} do
    wt = worktree(dir)
    line = "ok-before \e[8m ok-after"

    {out, _} = update(dir, wt, [{"APM_APPEND", line}])

    assert out =~ "ok-before"
    assert out =~ "ok-after"
    refute out =~ "\e[8m"
    refute out =~ "\e"
  end

  test "a C1 control in a changed line does not reach stdout raw", %{dir: dir} do
    wt = worktree(dir)
    line = "ok-before \u009b8m ok-after"

    {out, _} = update(dir, wt, [{"APM_APPEND", line}])

    assert out =~ "ok-before"
    assert out =~ "ok-after"
    refute out =~ <<0xC2, 0x9B>>
  end

  test "tabs and newlines in a changed line stay as they are", %{dir: dir} do
    wt = worktree(dir)

    {out, _} = update(dir, wt, [{"APM_APPEND", "col1\tcol2\nsecond-line"}])

    assert out =~ "+col1\tcol2\n+second-line"
  end

  defp commit_lock(wt) do
    git(wt, ["add", "apm.lock.yaml"])
    git(wt, ["commit", "-q", "-m", "lock"])
  end

  # A PATH holding only what the run needs, which has no gh on it.
  defp path_without_gh(dir) do
    dirs =
      for tool <- ["yq", "elixir", "erl"] do
        {out, 0} = System.cmd("mise", ["which", tool])
        Path.dirname(String.trim(out))
      end

    dirs = [Path.join(dir, "bin") | dirs] ++ ["/usr/bin", "/bin"]
    refute Enum.any?(dirs, &File.exists?(Path.join(&1, "gh"))), "fixture: gh is on the PATH"
    Enum.join(dirs, ":")
  end

  test "a lock that already differs from HEAD is not reported as no skill moved", %{dir: dir} do
    wt = worktree(dir)
    commit_lock(wt)
    lock_text = File.read!(Path.join(wt, "apm.lock.yaml"))
    File.write!(Path.join(wt, "apm.lock.yaml"), String.replace(lock_text, "abc", "def"))
    assert git_status(wt) =~ "apm.lock.yaml", "fixture: the lock does not differ from HEAD"

    {out, code} = update(dir, wt, [])

    refute out =~ "No skill moved", out
    assert out =~ ~r/uncommitted|already|differs|changes/i, out

    if File.exists?(Path.join(dir, "apm.log")),
      do: assert(code == 0, out),
      else: assert(code != 0, out)
  end

  defp git_status(wt) do
    {out, 0} = System.cmd("git", ["status", "--porcelain"], cd: wt)
    out
  end

  test "without gh the diff, the audit and the findings still come out", %{dir: dir} do
    wt = worktree(dir, [".claude/rules/probe.md"])
    write(wt, ".claude/rules/probe.md", "rule\n")
    commit_lock(wt)

    {out, code} =
      update(dir, wt, [{"PATH", path_without_gh(dir)}, {"APM_BUMP", "1"}, {"APM_AUDIT", "0"}])

    refute out =~ "ErlangError", out
    assert File.read!(Path.join(dir, "apm.log")) =~ "audit", "fixture: audit did not run\n" <> out
    assert out =~ "+changed", out
    assert out =~ "[fail] .claude/rules/probe.md", out
    assert code == 1, out
  end

  for audit <- ["0", "1"] do
    test "a clean skill exits by the audit alone (audit #{audit})", %{dir: dir} do
      wt = worktree(dir)

      {out, code} = update(dir, wt, [{"APM_AUDIT", unquote(audit)}])

      assert File.read!(Path.join(dir, "apm.log")) =~ "audit", "fixture: audit did not run"
      if unquote(audit) == "0", do: assert(code == 0, out), else: assert(code != 0, out)
    end
  end

  test "a moved revision whose age gh cannot give is shown as ?", %{dir: dir} do
    wt = worktree(dir)
    write(dir, "bin/gh", "#!/bin/sh\nexit 1\n")
    File.chmod!(Path.join(dir, "bin/gh"), 0o755)

    {out, _} = update(dir, wt, [{"APM_BUMP", "1"}])

    assert out =~ "Last change upstream", out
    assert out =~ "probe: ?", out
  end

  test "with no revision moved it says so", %{dir: dir} do
    wt = worktree(dir)

    {out, _} = update(dir, wt, [])

    assert out =~ "No skill moved", out
  end

  test "outside a linked worktree it refuses and never calls apm", %{dir: dir} do
    main = repo(dir)
    fake_apm(dir)

    {out, code} = run(dir, main, [])

    assert code == 1, out
    assert out =~ "Run this in a linked worktree"
    refute File.exists?(Path.join(dir, "apm.log"))
  end
end

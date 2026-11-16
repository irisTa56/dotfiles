# What the shared task upstream:pin-check must do, each case a repository it runs in,
# reading upstream repositories that the case's prepare sets up beside it.
defmodule Dotfiles.TaskCases.UpstreamPinCheck do
  @task "tasks/upstream/pin-check.ts"
  @identity ["-c", "user.name=t", "-c", "user.email=t@example.com"]

  defp marker(url, path, sha), do: "git::#{url}//#{path}?ref=#{sha}"

  # An upstream repository under `aux` on `main`, one commit per map of {path, content}
  # in `commits`, a nil content deleting the path; returns its URL and the commits.
  defp upstream(aux, name, commits) do
    dir = Path.join(aux, name)
    File.mkdir_p!(dir)
    git!(dir, ["init", "-q", "-b", "main"])

    shas =
      for files <- commits do
        Enum.each(files, fn {path, content} -> put(Path.join(dir, path), content) end)
        git!(dir, ["add", "-A"])
        git!(dir, @identity ++ ["commit", "-q", "--allow-empty", "-m", "c"])
        git!(dir, ["rev-parse", "HEAD"])
      end

    {"file://" <> dir, shas}
  end

  defp put(file, nil), do: File.rm!(file)

  defp put(file, content) do
    File.mkdir_p!(Path.dirname(file))
    File.write!(file, content)
  end

  # A bare repository under `aux` for the case's repository to push to.
  defp origin(aux) do
    dir = Path.join(aux, "origin.git")
    File.mkdir_p!(dir)
    git!(dir, ["init", "-q", "--bare", "-b", "main"])
    dir
  end

  # The case's git commands that commit its files and make them origin's main.
  defp publish(origin),
    do: [
      @identity ++ ["commit", "-qm", "base"],
      ["branch", "-M", "main"],
      ["remote", "add", "origin", origin],
      ["push", "-q", "origin", "main"]
    ]

  # What the task pushed, read after it ran; a miss is the task's to answer for, so it
  # comes back as a value the case reports rather than stopping the run.
  defp show(repo, object), do: read(repo, ["show", object])

  # The message of the commit `ref` names, read the same way.
  defp message(repo, ref), do: read(repo, ["log", "-1", "--format=%B", ref])

  defp read(repo, args) do
    case Dotfiles.Command.cmd("git", args,
           cd: repo,
           env: clear_git_env(),
           stderr_to_stdout: true
         ) do
      {out, 0} -> String.trim(out)
      {out, _} -> {:missing, String.trim(out)}
    end
  end

  # What a case's verify reports where the pushed commit's message lacks a part of
  # `present` or names a part of `absent`.
  defp pushed_message(origin, present, absent) do
    case message(origin, "upstream-pin-bump") do
      msg when is_binary(msg) ->
        for(part <- present, not String.contains?(msg, part), do: "the message lacks #{part}") ++
          for part <- absent, String.contains?(msg, part), do: "the message names #{part}"

      missing ->
        ["the pushed commit's message is #{inspect(missing)}"]
    end
  end

  defp holds(dir, path, want) do
    got = File.read!(Path.join(dir, path))
    if got == want, do: [], else: ["#{path} holds #{inspect(got)}"]
  end

  # Makes `files` origin's main from a repository of its own under `aux`, and returns the
  # git commands that leave the case's repository on that main with its own files staged
  # over it, so that the checkout differs from origin.
  defp diverge(aux, origin, files) do
    dir = Path.join(aux, "seed")
    File.mkdir_p!(dir)
    git!(dir, ["init", "-q", "-b", "main"])
    Enum.each(files, fn {path, content} -> put(Path.join(dir, path), content) end)
    git!(dir, ["add", "-A"])
    git!(dir, @identity ++ ["commit", "-qm", "base"])
    git!(dir, ["remote", "add", "origin", origin])
    git!(dir, ["push", "-q", "origin", "main"])

    [
      ["remote", "add", "origin", origin],
      ["fetch", "-q", "origin", "main"],
      ["reset", "-q", "origin/main"],
      ["add", "-A"]
    ]
  end

  # Setting up an upstream is the case's own doing, so a failure stops the run. A git
  # hook running this exports GIT_DIR and its kin, cleared here as the runner clears them.
  defp git!(dir, args) do
    {out, 0} =
      Dotfiles.Command.cmd("git", args, cd: dir, env: clear_git_env(), stderr_to_stdout: true)

    String.trim(out)
  end

  defp clear_git_env,
    do: for({name, _} <- System.get_env(), String.starts_with?(name, "GIT_"), do: {name, nil})

  def cases do
    [
      # A wrong pathspec, or a pin spelled so that it is not read, would otherwise pass
      # having checked nothing.
      %{
        name: "fails when no tracked file holds a pin",
        files: [{"Cargo.toml", "# upstream: git::https://example.com/a.git//lints?ref=main\n"}],
        exit: 2,
        output: ["found no upstream pin"]
      },
      # Most commits to an upstream leave the pinned path alone, and a check failing
      # on each of them would get ignored.
      %{
        name: "passes a pin whose path is unchanged on the default branch's tip",
        prepare: fn aux ->
          {url, [pin, _tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"README.md" => "r"}])

          %{files: [{"Cargo.toml", "# upstream: #{marker(url, "lints", pin)}\n"}]}
        end,
        exit: 0,
        output: ["[ok]"]
      },
      # The pin may be ahead of the default branch as well as behind it, so the
      # report names both commits without saying which is older.
      %{
        name: "fails on a pin whose path changed, naming both commits and the file",
        prepare: fn aux ->
          {url, [pin, tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          %{
            files: [{"Cargo.toml", "# upstream: #{marker(url, "lints", pin)}\n"}],
            output: ["#{url}//lints differs", "pinned: #{pin}", "tip:    #{tip}", "Cargo.toml"]
          }
        end,
        exit: 1
      },
      # Removing the path upstream is a change to it like any other.
      %{
        name: "fails on a pin whose path is gone from the default branch",
        prepare: fn aux ->
          {url, [pin, _tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => nil}])

          %{files: [{"Cargo.toml", "# #{marker(url, "lints", pin)}\n"}]}
        end,
        exit: 1,
        output: ["is gone from the default branch"]
      },
      %{
        name: "fails as unable to tell where the upstream cannot be read",
        prepare: fn aux ->
          url = "file://" <> Path.join(aux, "missing")
          %{files: [{"Cargo.toml", "# #{marker(url, "lints", String.duplicate("a", 40))}\n"}]}
        end,
        exit: 2,
        output: ["could not read the default branch of file://"]
      },
      # A pin taken from a branch since deleted, or squash-merged, names a commit the
      # upstream no longer has.
      %{
        name: "fails as unable to tell where the pinned commit is not in the upstream",
        prepare: fn aux ->
          {url, [_]} = upstream(aux, "up", [%{"lints/a.toml" => "a"}])
          missing = String.duplicate("a", 40)
          %{files: [{"Cargo.toml", "# #{marker(url, "lints", missing)}\n"}]}
        end,
        exit: 2,
        output: ["could not read #{String.duplicate("a", 40)} from file://"]
      },
      %{
        name: "fails as unable to tell where the path is not in the pinned commit",
        prepare: fn aux ->
          {url, [pin]} = upstream(aux, "up", [%{"lints/a.toml" => "a"}])
          %{files: [{"Cargo.toml", "# #{marker(url, "lint", pin)}\n"}]}
        end,
        exit: 2,
        output: ["//lint does not exist at the pinned"]
      },
      %{
        name: "reads only the files a pathspec argument names",
        prepare: fn aux ->
          {url, [old, tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          %{
            files: [
              {"Cargo.toml", "# #{marker(url, "lints", old)}\n"},
              {"mise.toml", "includes = [\"#{marker(url, "lints", tip)}\"]\n"}
            ]
          }
        end,
        args: ["mise.toml"],
        exit: 0,
        output: ["[ok]"]
      },
      # The same pin spelled in two files is one upstream question, answered once.
      %{
        name: "reports each pin, a pin spelled in two files once, and fails if any differs",
        prepare: fn aux ->
          {tasks, [tasks_pin]} = upstream(aux, "tasks", [%{"tasks/a.sh" => "a"}])

          {lints, [lints_pin, _]} =
            upstream(aux, "lints", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          %{
            files: [
              {"mise.toml", "includes = [\"#{marker(tasks, "tasks", tasks_pin)}\"]\n"},
              {"Cargo.toml", "# #{marker(lints, "lints", lints_pin)}\n"},
              {"clippy.toml", "# #{marker(lints, "lints", lints_pin)}\n"}
            ],
            output: [
              "[ok] #{tasks}//tasks",
              "#{lints}//lints differs from the default branch, pinned in Cargo.toml, clippy.toml"
            ]
          }
        end,
        exit: 1
      },
      %{
        name: "with --bump, moves a differing pin to the tip in every file that spells it",
        prepare: fn aux ->
          {url, [old, tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          {other, [other_pin]} = upstream(aux, "other", [%{"x/a" => "a"}])
          kept = marker(other, "x", other_pin)

          %{
            files: [
              {"Cargo.toml", "# #{marker(url, "lints", old)}\n# #{kept}\n"},
              {"clippy.toml", "# #{marker(url, "lints", old)}\n"}
            ],
            verify: fn dir ->
              expect = %{
                "Cargo.toml" => "# #{marker(url, "lints", tip)}\n# #{kept}\n",
                "clippy.toml" => "# #{marker(url, "lints", tip)}\n"
              }

              for {path, want} <- expect,
                  File.read!(Path.join(dir, path)) != want,
                  do: "#{path} holds #{inspect(File.read!(Path.join(dir, path)))}"
            end,
            output: ["moved the pin of #{url}//lints to #{tip}"]
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", nil}],
        exit: 1
      },
      # The checkout is often a pull request's merge commit, so the move is made on the
      # default branch as origin has it, and the working tree is left alone.
      %{
        name:
          "with --bump under GitHub Actions, pushes the move on origin's default branch to upstream-pin-bump",
        prepare: fn aux ->
          {url, [old, tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          origin = origin(aux)
          pinned = "# #{marker(url, "lints", old)}\n"

          %{
            files: [{"Cargo.toml", pinned}, {"README.md", "r"}],
            git: publish(origin),
            verify: fn dir ->
              want = "# #{marker(url, "lints", tip)}"

              [
                {File.read!(Path.join(dir, "Cargo.toml")), pinned,
                 "the working tree's Cargo.toml"},
                {show(origin, "upstream-pin-bump:Cargo.toml"), want, "the pushed Cargo.toml"},
                {show(origin, "upstream-pin-bump:README.md"), "r", "the pushed README.md"},
                {show(origin, "upstream-pin-bump~1"), show(origin, "main"),
                 "the pushed commit's parent"}
              ]
              |> Enum.flat_map(fn {got, want, what} ->
                if got == want, do: [], else: ["#{what} is #{inspect(got)}"]
              end)
            end,
            output: [
              "pushed the move to upstream-pin-bump",
              "#{url}//lints to #{tip}",
              "/compare/upstream-pin-bump?expand=1"
            ]
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", "true"}],
        exit: 1
      },
      %{
        name: "with --bump under GitHub Actions, leaves an existing upstream-pin-bump alone",
        prepare: fn aux ->
          {url, [old, _tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          origin = origin(aux)

          %{
            files: [{"Cargo.toml", "# #{marker(url, "lints", old)}\n"}],
            git: publish(origin) ++ [["push", "-q", "origin", "main:upstream-pin-bump"]],
            verify: fn _ ->
              if show(origin, "upstream-pin-bump") == show(origin, "main"),
                do: [],
                else: ["upstream-pin-bump moved"]
            end
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", "true"}],
        exit: 1,
        output: ["upstream-pin-bump already exists on origin"]
      },
      # A pin added on a pull request's branch is not on the default branch yet.
      %{
        name:
          "with --bump under GitHub Actions, fails as unable to tell where the default branch holds no pin to move",
        prepare: fn aux ->
          {url, [old, _tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          origin = origin(aux)

          # The pin is staged only after origin got main without it.
          %{
            files: [{"README.md", "r"}, {"Cargo.toml", "# #{marker(url, "lints", old)}\n"}],
            git:
              [["rm", "-q", "--cached", "Cargo.toml"]] ++
                publish(origin) ++ [["add", "Cargo.toml"]]
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", "true"}],
        exit: 2,
        output: ["holds none of these pins to move"]
      },
      # The move would leave a pin at a commit that does not hold the path, which the
      # next check reports as unable to tell.
      %{
        name: "with --bump, leaves a pin whose path is gone from the default branch where it was",
        prepare: fn aux ->
          {url, [old, _tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => nil}])

          pinned = "# #{marker(url, "lints", old)}\n"

          %{
            files: [{"Cargo.toml", pinned}],
            verify: fn dir -> holds(dir, "Cargo.toml", pinned) end
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", nil}],
        exit: 1,
        output: []
      },
      # Only a marker with a path is a pin; a git URL without one is some other string.
      %{
        name: "reads no pin from a git URL that has no path",
        prepare: fn aux ->
          {url, [pin, _tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"README.md" => "r"}])

          sha = String.duplicate("a", 40)

          %{
            files: [
              {"Cargo.toml",
               "# git::https://example.com/a/b.git?ref=#{sha}\n# #{marker(url, "lints", pin)}\n"}
            ]
          }
        end,
        exit: 0,
        output: ["[ok]"]
      },
      # A URL that opens with a dash would be read by git as an option of its own.
      %{
        name: "reads no pin from a URL git would take for an option",
        prepare: fn aux ->
          {url, [pin, _tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"README.md" => "r"}])

          sha = String.duplicate("a", 40)

          %{
            files: [
              {"Cargo.toml",
               "# git::--upload-pack=touch-me//p?ref=#{sha}\n# #{marker(url, "lints", pin)}\n"}
            ]
          }
        end,
        exit: 0,
        output: ["[ok]"]
      },
      # The checkout is often a pull request's merge commit, so its pin can be one that
      # origin's default branch no longer holds; the move starts where the default
      # branch's file is.
      %{
        name:
          "with --bump under GitHub Actions, names the commit origin's default branch pins as where the move starts",
        prepare: fn aux ->
          {url, [x, y, _tip]} =
            upstream(aux, "up", [
              %{"lints/a.toml" => "a"},
              %{"lints/a.toml" => "b"},
              %{"lints/a.toml" => "c"}
            ])

          origin = origin(aux)

          %{
            files: [{"Cargo.toml", "# #{marker(url, "lints", x)}\n"}],
            git: diverge(aux, origin, [{"Cargo.toml", "# #{marker(url, "lints", y)}\n"}]),
            verify: fn _ -> pushed_message(origin, [y], [x]) end
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", "true"}],
        exit: 1,
        output: []
      },
      %{
        name:
          "with --bump under GitHub Actions, claims no move of a pin origin's default branch does not hold",
        prepare: fn aux ->
          {a, [a_old, _]} = upstream(aux, "a", [%{"x/a" => "a"}, %{"x/a" => "b"}])
          {b, [b_old, _]} = upstream(aux, "b", [%{"y/a" => "a"}, %{"y/a" => "b"}])
          origin = origin(aux)

          %{
            files: [
              {"Cargo.toml", "# #{marker(a, "x", a_old)}\n# #{marker(b, "y", b_old)}\n"}
            ],
            git: diverge(aux, origin, [{"Cargo.toml", "# #{marker(a, "x", a_old)}\n"}]),
            verify: fn _ -> pushed_message(origin, [a], [b]) end
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", "true"}],
        exit: 1,
        output: []
      },
      %{
        name: "fails as unable to tell on an option it does not take",
        files: [],
        args: ["--frobnicate"],
        exit: 2,
        output: ["takes --bump"]
      }
    ]
    |> Enum.map(&Map.put(&1, :task, @task))
  end
end

# What the shared task upstream:pin-check must do, each case a repository it runs in,
# reading upstream repositories that the case's prepare sets up beside it.
defmodule Dotfiles.TaskCases.UpstreamPinCheck do
  @task "tasks/upstream/pin-check.sh"
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

  # The bytes of `object` in `repo` exactly as git holds them, unlike show/2, which
  # trims; a miss comes back as a value the case reports.
  defp show_bytes(repo, object), do: output(repo, ["show", object])

  # The message of the commit `ref` names, read the same way.
  defp message(repo, ref), do: read(repo, ["log", "-1", "--format=%B", ref])

  defp read(repo, args) do
    with out when is_binary(out) <- output(repo, args), do: String.trim(out)
  end

  defp output(repo, args) do
    case Dotfiles.Command.cmd("git", args,
           cd: repo,
           env: Dotfiles.TaskCases.cleared_git_env(),
           stderr_to_stdout: true
         ) do
      {out, 0} -> out
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

      {:missing, _} = missing ->
        ["the pushed commit's message is #{inspect(missing)}"]
    end
  end

  # What a case's verify reports where origin's upstream-pin-bump does not hold `want`
  # in `path`, the branch missing included.
  defp bump_holds(origin, path, want) do
    want = String.trim(want)

    case show(origin, "upstream-pin-bump:#{path}") do
      ^want ->
        []

      {:missing, out} ->
        ["upstream-pin-bump has no #{path}: #{out}"]

      got when is_binary(got) ->
        ["upstream-pin-bump's #{path} ends #{inspect(String.slice(got, -120, 120))}"]
    end
  end

  # What a case's verify reports where origin has an upstream-pin-bump at all.
  defp no_bump_pushed(origin) do
    case show(origin, "upstream-pin-bump") do
      {:missing, _} -> []
      pushed when is_binary(pushed) -> ["upstream-pin-bump was pushed"]
    end
  end

  # What a case's verify reports where the task fetched into `repo`.
  defp not_fetched_into(repo) do
    if File.exists?(Path.join(repo, "FETCH_HEAD")),
      do: ["#{repo} was fetched into"],
      else: []
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

  # Setting up an upstream is the case's own doing, so a failure stops the run.
  defp git!(dir, args) do
    {out, 0} =
      Dotfiles.Command.cmd("git", args,
        cd: dir,
        env: Dotfiles.TaskCases.cleared_git_env(),
        stderr_to_stdout: true
      )

    String.trim(out)
  end

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
      # git grep exits 1 where no tracked file spells git:: at all, which is no failure of
      # git's, so the run says the same as for a marker it does not read.
      %{
        name: "fails when no tracked file spells a marker at all",
        files: [{"Cargo.toml", "[package]\n"}],
        exit: 2,
        output: "[fail] found no upstream pin in the tracked files\n"
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
      # A git hook, such as a pre-push one that runs this check, exports GIT_DIR for the
      # repository it runs for, which would point the scratch repository's runs at it.
      %{
        name: "leaves the repository of a git hook it runs under alone",
        prepare: fn aux ->
          {url, [pin, _tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"README.md" => "r"}])

          hook = Path.join(aux, "hook.git")
          git!(aux, ["init", "-q", "--bare", hook])

          %{
            files: [{"Cargo.toml", "# upstream: #{marker(url, "lints", pin)}\n"}],
            env: [{"GIT_DIR", hook}],
            verify: fn _ -> not_fetched_into(hook) end
          }
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
        # git's own reason comes first, so the run says why it could not tell.
        output: [
          "does not appear to be a git repository",
          "could not read the default branch of file://"
        ]
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
      # The link a person follows from the bump is GitHub's comparison of the two
      # commits, and the commit is the bot's, since a runner has no git identity of its
      # own; the upstream is reached at a github.com URL that git's config rewrites to
      # a local one.
      %{
        name:
          "with --bump under GitHub Actions, commits as the bot and links GitHub's comparison",
        prepare: fn aux ->
          {local, [old, tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          url = "https://github.com/o/up.git"
          config = Path.join(aux, "gitconfig")
          File.write!(config, "[url \"#{local}\"]\n\tinsteadOf = #{url}\n")
          origin = origin(aux)

          %{
            files: [{"Cargo.toml", "# #{marker(url, "lints", old)}\n"}],
            git: publish(origin),
            env: [
              {"GITHUB_ACTIONS", "true"},
              {"GITHUB_SERVER_URL", "https://github.com"},
              {"GITHUB_REPOSITORY", "o/consumer"},
              {"GIT_CONFIG_GLOBAL", config},
              {"GIT_CONFIG_NOSYSTEM", "1"}
            ],
            verify: fn _ ->
              bot = "github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>"

              case read(origin, ["log", "-1", "--format=%an <%ae>", "upstream-pin-bump"]) do
                ^bot -> []
                got -> ["the pushed commit's author is #{inspect(got)}"]
              end
            end,
            output: [
              "what it brings: https://github.com/o/up/compare/#{old}...#{tip}",
              "open its pull request: https://github.com/o/consumer/compare/upstream-pin-bump?expand=1"
            ]
          }
        end,
        args: ["--bump"],
        exit: 1
      },
      # A file that is not UTF-8 text may still hold a pin; moving the pin must leave every
      # other byte of it as it was.
      %{
        name: "with --bump, keeps every byte of a file outside the pin it moves",
        prepare: fn aux ->
          {url, [old, tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          %{
            files: [{"Cargo.toml", "# caf\xe9\n# #{marker(url, "lints", old)}\n# \xff\xfe\n"}],
            verify: fn dir ->
              holds(
                dir,
                "Cargo.toml",
                "# caf\xe9\n# #{marker(url, "lints", tip)}\n# \xff\xfe\n"
              )
            end
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", nil}],
        exit: 1,
        output: []
      },
      %{
        name:
          "with --bump under GitHub Actions, pushes every byte of a file outside the pin it moves",
        prepare: fn aux ->
          {url, [old, tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          origin = origin(aux)

          %{
            files: [{"Cargo.toml", "# caf\xe9\n# #{marker(url, "lints", old)}\n# \xff\xfe\n"}],
            git: publish(origin),
            verify: fn _ ->
              want = "# caf\xe9\n# #{marker(url, "lints", tip)}\n# \xff\xfe\n"

              case show_bytes(origin, "upstream-pin-bump:Cargo.toml") do
                ^want -> []
                got -> ["the pushed Cargo.toml is #{inspect(got, binaries: :as_binaries)}"]
              end
            end
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", "true"}],
        exit: 1,
        output: []
      },
      # Later steps of the job may read history from the checkout, so the bump must leave
      # how much of it the repository holds as it found it.
      %{
        name: "with --bump under GitHub Actions, leaves a full checkout's history as it was",
        prepare: fn aux ->
          {url, [old, _tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          origin = origin(aux)

          %{
            files: [{"Cargo.toml", "# #{marker(url, "lints", old)}\n"}],
            git:
              publish(origin) ++
                [
                  @identity ++ ["commit", "-q", "--allow-empty", "-m", "second"],
                  @identity ++ ["commit", "-q", "--allow-empty", "-m", "third"]
                ],
            verify: fn dir ->
              shallow = git!(dir, ["rev-parse", "--is-shallow-repository"])
              count = git!(dir, ["rev-list", "--count", "HEAD"])

              if {shallow, count} == {"false", "3"},
                do: [],
                else: [
                  "the checkout reports shallow=#{shallow} with #{count} commits; wanted false, 3"
                ]
            end
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", "true"}],
        exit: 1,
        output: []
      },
      # A pin it could not tell about leaves the run unable to tell, while the bump still
      # moves the pin that differs.
      %{
        name:
          "with --bump, moves a differing pin and still fails as unable to tell beside an unreadable one",
        prepare: fn aux ->
          {url, [old, tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          unreadable =
            "# #{marker("file://" <> Path.join(aux, "missing"), "x", String.duplicate("a", 40))}\n"

          %{
            files: [{"Cargo.toml", "# #{marker(url, "lints", old)}\n" <> unreadable}],
            verify: fn dir ->
              holds(dir, "Cargo.toml", "# #{marker(url, "lints", tip)}\n" <> unreadable)
            end,
            output: ["moved the pin of #{url}//lints to #{tip}"]
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", nil}],
        exit: 2
      },
      # A token without contents: write, or a fork's pull request, gets the push refused.
      %{
        name:
          "with --bump under GitHub Actions, fails as unable to tell where origin refuses the push",
        prepare: fn aux ->
          {url, [old, _tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          origin = origin(aux)
          hook = Path.join(origin, "hooks/pre-receive")

          File.write!(hook, """
          #!/bin/sh
          while read -r _ _ ref; do [ "$ref" = refs/heads/upstream-pin-bump ] && exit 1; done
          exit 0
          """)

          File.chmod!(hook, 0o755)

          %{
            files: [{"Cargo.toml", "# #{marker(url, "lints", old)}\n"}],
            git: publish(origin),
            verify: fn _ -> no_bump_pushed(origin) end
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", "true"}],
        exit: 2,
        output: ["could not push the move to upstream-pin-bump"]
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
      # A ref longer than a sha-1, such as a sha-256, is not one this compares; reading
      # its first 40 digits as one would pin a commit nobody named.
      %{
        name: "reads no pin from a ref with more hex digits than a sha-1",
        files: [
          {"Cargo.toml",
           "# git::https://example.com/a.git//lints?ref=#{String.duplicate("0123456789abcdef", 4)}\n"}
        ],
        exit: 2,
        output: ["found no upstream pin"]
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
      # A file larger than a pipe's buffer is read back from git and hashed again whole,
      # so the move must not stall on its size or leave that file's pin behind.
      %{
        name:
          "with --bump under GitHub Actions, moves a pin in a large file beside one in a small file",
        prepare: fn aux ->
          {url, [old, tip]} =
            upstream(aux, "up", [%{"lints/a.toml" => "a"}, %{"lints/a.toml" => "b"}])

          origin = origin(aux)
          padding = "# " <> String.duplicate("x", 1_600_000) <> "\n"

          %{
            files: [
              {"big.toml", padding <> "# #{marker(url, "lints", old)}\n"},
              {"Cargo.toml", "# #{marker(url, "lints", old)}\n"}
            ],
            git: publish(origin),
            verify: fn _ ->
              bump_holds(origin, "big.toml", padding <> "# #{marker(url, "lints", tip)}\n")
            end
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", "true"}],
        exit: 1,
        output: []
      },
      # What matters is the pin origin holds, whose tree may equal the tip's already.
      %{
        name:
          "with --bump under GitHub Actions, pushes no move of a pin origin holds at a commit whose path equals the tip's",
        prepare: fn aux ->
          {url, [x, y, _tip]} =
            upstream(aux, "up", [
              %{"lints/a.toml" => "a"},
              %{"lints/a.toml" => "b"},
              %{"README.md" => "r"}
            ])

          origin = origin(aux)

          %{
            files: [{"Cargo.toml", "# #{marker(url, "lints", x)}\n"}],
            git: diverge(aux, origin, [{"Cargo.toml", "# #{marker(url, "lints", y)}\n"}]),
            verify: fn _ -> no_bump_pushed(origin) end
          }
        end,
        args: ["--bump"],
        env: [{"GITHUB_ACTIONS", "true"}],
        exit: 2,
        output: ["holds none of these pins to move"]
      },
      %{
        name: "fails as unable to tell where it cannot make its scratch repository",
        prepare: fn aux ->
          {url, [pin]} = upstream(aux, "up", [%{"lints/a.toml" => "a"}])

          %{
            files: [{"Cargo.toml", "# #{marker(url, "lints", pin)}\n"}],
            env: [{"TMPDIR", Path.join(aux, "no-such-dir")}]
          }
        end,
        exit: 2,
        output: []
      },
      # Installing the release can fail, for a version not yet published or a host no
      # binary is built for, and that is no answer about the pins.
      %{
        name: "fails as unable to tell where the release it pins cannot be installed",
        files: [
          {"Cargo.toml", "# git::https://example.com/u.git//p?ref=#{String.duplicate("a", 40)}\n"}
        ],
        env: [{"TASK_CASE_MISE_INSTALL_STATUS", "1"}],
        exit: 2,
        output: ""
      },
      %{
        name: "fails as unable to tell on an option it does not take",
        files: [],
        args: ["--frobnicate"],
        exit: 2,
        output: ["takes --bump"]
      }
    ]
    |> Enum.map(&Map.merge(&1, %{task: @task}))
  end
end

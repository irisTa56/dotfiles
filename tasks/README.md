# Shared tasks

`tasks/` holds the repository-agnostic mise tasks this repository lends to others, each in a section below:

- `secrets:commit-scan`, which gates a commit here too
- `secrets:push-scan`, which gates a push here too
- `link-check:staged`, which gates a commit here too
- `skills:size-check`
- `upstream:pin-check`

This repository checks its own skills, staged links and pushes with the subcommands `skills:size-check`, `link-check:staged` and `secrets:push-scan` run, built from the working tree rather than installed from the release the task pins, since a release pull request pins one not yet published.
The first half of this file is for a repository taking them, the second for whoever adds the next one.

## Taking them

A consuming repository picks them up with a [`task_config.includes`](https://mise.jdx.dev/tasks/task-configuration.html#task_config.includes) entry pointing here, and needs no `[tools]` of its own: a task that needs a tool mise can install declares it, and mise installs it for that task alone.
A job that sets [`MISE_TASK_RUN_AUTO_INSTALL=false`](https://mise.jdx.dev/configuration/settings.html#task.run_auto_install) turns that install off, so a task declaring a tool fails there unless the job installs the tool itself; a task that runs the `dotfiles-tasks` binary installs it itself with `mise install`, so it runs either way unless it declares a tool as well, as `link-check:staged` does lychee and `secrets:push-scan` trufflehog.
Beyond that the library assumes git and bash.
A task that runs the `dotfiles-tasks` binary, which every one of them but `secrets:commit-scan` does, also needs a release built for the host: one is built for macOS on Apple silicon and for Linux on x86-64 and ARM64, and none for an Intel Mac or Windows.
Such a task prints its usage on `--help` after mise's `--`, as in `mise run link-check:staged -- --help`, and exits 0 having done nothing else; it exits 2 on an argument it does not take, saying which.

```toml
# mise.toml, in the consuming repository
[task_config]
includes = [
  "git::https://github.com/irisTa56/dotfiles.git//tasks?ref=<40-character commit sha>",
  "mise-tasks", # and whichever other default directories that repository uses
]
```

mise marks a `git::` include experimental, so the syntax and the caching below carry no compatibility promise and a consumer wired to several of these is worth keeping an eye on mise's releases for.
Naming `includes` replaces the default file-task directories rather than adding to them, so a repository that keeps its own tasks in one has to list it back.
Of two entries defining the same task the later wins, which is how a repository overrides a shared one: it puts its own file task, named `<name>.sh` as well, under a directory listed after this include.
An extensionless one is a task of another name, since mise [keeps the extension in a file task's name](https://github.com/jdx/mise/blob/v2026.9.15/src/config/mod.rs#L5491-L5492), dropping it only for display.
It takes the bare name whatever the order, which `mise run <name>` [resolves to ahead of the extension-stripped match](https://github.com/jdx/mise/blob/v2026.9.15/src/task/mod.rs#L3956-L3961), and the shared task stays runnable as `<name>.sh`.

An inline `[tasks.<name>]` in the consuming repository [configures the shared task under either name](https://mise.jdx.dev/tasks/task-discovery.html#configuring-file-tasks-from-toml).
Declare only metadata in it, such as a `description`, an `env` or `depends`: the shared script stays the task's command and runs after whatever `depends` names.
A block carrying `run`, `run_windows` or `file` replaces the shared task instead, and `<name>.sh` stops existing as well, which makes it an override by another route rather than a way to configure the shared one.

Pin `ref` to a commit on `main`, not a branch, and spell it in full.
For a sha mise clones the whole repository and checks the commit out, so a commit no branch reaches is not in the clone: a pin taken from a branch since deleted, or from a pull request this repository squash-merged, fails the next time the cache is cold rather than when it was written.
A failing include takes every task in the consuming repository with it, that repository's own included.
A task that runs the `dotfiles-tasks` binary fails instead, at mise's install, on a commit whose wrapper names a release not yet published: the commit that merges a release moves the wrapper, and the release is published once its binaries are built, minutes later, so a pin taken in between, by hand or by `--bump`, works from then on.
mise [keys its clone cache on the repository URL and the ref alone, and reuses an existing clone without fetching](https://github.com/jdx/mise/blob/v2026.9.11/src/task/task_file_providers/remote_task_git.rs), so a branch ref stays at whatever it first resolved to; a new commit sha is a new key and clones afresh.
It recognises a sha only at 40 characters and treats anything shorter as a branch or tag name, which fails with "the remote didn't have any ref that matched" rather than with anything about its length.
That link is pinned to a release for the same reason the `ref` is: on `main` it would keep resolving while quietly ceasing to support the claim.

A workflow that runs them sets `MISE_NO_HOOKS: "1"` in its workflow-wide `env`, since mise runs the consuming repository's [`[hooks] postinstall`](https://mise.jdx.dev/hooks.html) on every `mise install` there:

- the one [`jdx/mise-action` runs unless told `install: false`](https://github.com/jdx/mise-action/blob/v5.2.10/action.yml#L37-L40);
- the one `mise run` runs to install a tool a task declares.

A `postinstall` that sets up a local checkout has nothing to do in CI and can break a job there: a pre-push hook it registers runs on the push `upstream:pin-check -- --bump` makes, and where the hook fails, as one whose tool the job did not install does, the push fails with it.
The `mise install` a task runs for the `dotfiles-tasks` binary skips the hooks itself.

### `secrets:commit-scan`

It reads the staged diff, so it belongs in a pre-commit hook.
A plain checkout stages nothing and it passes having scanned nothing, which is not a CI gate.

### `secrets:push-scan`

It runs trufflehog over the commits a push is about to send, and fails the push on a hit.
trufflehog walks the ref's whole history and the task answers for the part of it the push would send, so the wait scales with the repository rather than with the push: a second or so over a few hundred commits, four over ten thousand.
`secrets:commit-scan` gates each commit, but gitleaks' default rules miss a credential embedded in a connection string or a URL, and so does [GitHub's push protection](https://docs.github.com/en/code-security/secret-scanning/introduction/supported-secret-scanning-patterns) as it comes: by default it covers provider tokens, the generic connection-string patterns only once configured, and a URL carrying `user:pass@` under no pattern at all.
What trufflehog closes of that gap is neither a class of credential nor the [detectors it ships](https://github.com/trufflesecurity/trufflehog/tree/v3.97.5/pkg/detectors): on 3.97.5, `mssql://user:pass@host` goes unreported although a `sqlserver` detector is there, while the same credential written as `Server=…;Password=…;` is reported.
So a push this passes is not an assurance that some particular form was looked for.

It reads the pushed refs from stdin and takes the remote's name from its first argument, so a pre-push hook is the only caller it works for.
mise writes one, resolving the shared hooks directory from a linked worktree as well:

```shell
mise generate git-pre-commit --hook pre-push --task secrets:push-scan --write
```

Which repositories are scanned is which repositories have that hook, so there is no list of them to keep anywhere.
Every branch and worktree of the repository shares the hook, which runs whichever `secrets:push-scan` mise finds from the pushing checkout.
One whose own tasks predate it (an older pin in a consumer, an older `tasks/` here) runs the copy of any checkout it sits inside, and otherwise fails with mise's `no task secrets:push-scan found` until it brings in a default branch that has the task.
The command moves an existing `pre-push` aside as `pre-push.old`, so where another tool owns that hook, it stops the tool's hook from running, and a tool that chains to a moved-aside hook may do so only if the file was there when it wrote its own, leaving pushes unscanned with nothing printed.
There, register the task as a hook in git's config instead, which leaves the hook file alone:

```shell
git config hook.secrets-push-scan.event pre-push
git config hook.secrets-push-scan.command 'mise run secrets:push-scan'
```

git runs it before the file under the hooks directory, with the same arguments and stdin, and refuses the push when it fails.
Running it again changes nothing, and it lands in the config every worktree shares.
It needs git 2.54, [the release that added hooks defined in config](https://github.com/git/git/blob/v2.54.0/Documentation/RelNotes/2.54.0.adoc); an older git ignores the keys and scans nothing, and `git hook list pre-push` naming `secrets-push-scan` is what shows it will run.
A push from an environment with no mise on PATH, an editor's Git UI say, fails on either hook rather than going out unscanned.

It exits 1 on a hit, and 2 where the release it runs cannot be installed or no remote's name reaches it; a scan that could not answer exits non-zero as well, with git's own status where a git command failed, and any of these refuses the push.

### `link-check:staged`

It runs lychee on the files the commit stages, so it belongs in a pre-commit hook, and fails the commit on a broken link.
It takes the staged files among those `lychee .` would read there, so the repository's `lychee.toml` (`extensions`, `exclude_path`) and `.gitignore` decide which files count, as they do for a run over the whole tree.
Every link in such a file is checked, so an old link in a staged file can fail the commit as well; a link in an unstaged file never does.
The files are chosen from the index but read from disk, as `lychee .` reads them, so a staged file with edits left unstaged — part of it staged with `git add -p`, or edited again after `git add` — is checked as it is on disk: a broken link only in those edits fails the commit, and one staged but since removed on disk passes it.
A file whose name contains a newline, or is not UTF-8, is not matched, and goes unchecked.
It runs at the repository root, so lychee runs under that repository's `lychee.toml`, whose [settings](https://lychee.cli.rs/guides/config/) decide how links are checked — whether over the network, how long a result is cached, which fragments are looked for — with `--cache` added.
lychee writes its [request cache](https://lychee.cli.rs/recipes/caching/), `.lycheecache`, there, so the repository has to ignore it; it lives in each checkout, so a new worktree starts with it empty.

It takes no argument.
It exits 1 on a broken link, and 2 on an argument or where the release it runs cannot be installed; either blocks the commit.
A broken link prints the variable that skips this task alone, `MISE_TASK_SKIP=link-check:staged.sh`, set on the `git commit` it blocked, and the variable skips it on a failed install too.
The full name is what matters: mise skips nothing for `link-check:staged` without the `.sh`, for the reason the section above on overriding a task gives.
The bypass otherwise at hand is `--no-verify`, which drops every other pre-commit check with it.

### `skills:size-check`

It fails where the body of a skill git tracks, its frontmatter left out, is longer than what Claude Code re-attaches of an invoked skill after a compaction, and names each such `SKILL.md` with its length.
A rule past that point is missing from the session once a compaction has run, so a skill over the limit moves what it reads only at one moment into files it reads by path.
How the limit is set and counted is in `rust/src/skills_size_check.rs`, the `dotfiles-tasks` subcommand the task runs.

It reads the skills under `.claude/skills/`, and a repository that keeps them elsewhere names them by a git pathspec as its argument, such as `mise run skills:size-check -- ':(glob)skills/*/SKILL.md'`; without `glob`, `*` also matches across directories.
A skill that passes prints nothing, so a commit's output stays quiet; `--verbose` before the pathspec prints each passing skill's length against the limit too, such as `mise run skills:size-check -- --verbose ':(glob)skills/*/SKILL.md'`, for sizing a skill rather than measuring it by hand.
It asks git for the list, so an ignored skill, such as one a package manager installs, is left out, and a skill staged for the commit is checked.
It exits 1 where git lists no skill, so a wrong pathspec fails rather than passing having read nothing, and where a `SKILL.md` opens a frontmatter it never closes; one that opens none is counted whole.
A `SKILL.md` it cannot read, in a directory it may not search or behind a symlink that loops among them, or one that is not UTF-8, fails as well, since its length is unknown, and the skills after it are still read.
It reads the working tree whole rather than the staged diff, so it serves as a CI gate as well as in a pre-commit hook, and passes over a skill deleted there but not yet from the index.
It exits 2 on an argument past the pathspec, and where the release it runs cannot be installed.

### `upstream:pin-check`

It says whether each path a repository pins in an upstream repository is the same at the pinned commit as on the tip of that upstream's default branch.
It is for a repository that took something from an upstream at one commit and wants to hear when that thing changes there, without following the upstream automatically.

A pin is a marker of the form `git::<url>//<path>?ref=<40-character commit sha>` anywhere in a file git tracks, a comment included, where `<url>` opens with its scheme, such as `https://`:

```toml
# Cargo.toml: the lints taken in part from an upstream
# upstream: git::https://github.com/morganlinton/BeautifulRust.git//lints?ref=<40-character commit sha>
```

A `task_config.includes` entry is in that form already, so the pin [Taking them](#taking-them) asks for is checked as one of these, with no marker to add.
It reads every tracked file by default; git pathspecs as arguments narrow that, such as `mise run upstream:pin-check -- mise.toml Cargo.toml`.
A pin spelled in several files is checked once, and a difference names each file that spells it.

It compares what the two commits carry under the path rather than the commits, so an upstream that moved without touching the path, which is most of its commits, asks for no bump.
It compares the upstream with itself and never with the repository's own files, so a repository that took only part of the path, or edited what it took, still passes until the upstream changes.
It exits 1 when a pinned path differs and names both commits, without saying which is the older, since the pin may be ahead of the default branch as well as behind it.
It exits 1 too where the path is gone from the default branch, and says so.
It exits 2 when it could not tell instead, and says why:

- the release of `dotfiles-tasks` it runs could not be installed
- no tracked file, or none the pathspecs name, holds a pin
- an upstream it could not read, or a pinned commit that upstream does not have
- a path the pinned commit does not have
- a scratch directory or repository it could not create

It does not tell whether a pin is a commit on the default branch: one that no branch reaches but carries the same path passes, which for an include fails only once mise's cache is cold, as [the pinning rule above](#taking-them) warns.
Where some pins differ and others it could not tell, it exits 2.

Give it a CI step of its own and fail the build on anything but 0, so a failure costs a re-run and nothing half-done.
The task runs a released `dotfiles-tasks` binary, which its script pins, installs with `mise install` from this repository's GitHub Releases and runs from the directory `mise where` prints, so the job needs nothing beyond mise and git, and curl for the pull request `--bump` opens.

With `--bump` before any pathspec, where it would exit 1 it first moves each differing pin to its upstream's tip, except one whose path is gone there, and prints what the move brings, a GitHub URL comparing the two commits where the upstream is on GitHub, then exits 1 all the same.
Run by hand, it rewrites each pin in the files that spell it, for whoever ran it to commit.

Under GitHub Actions it leaves the working tree alone, which is often a pull request's merge commit, and proposes the move as a pull request instead, as `release-please` proposes a release:

- Where no `upstream-pin-bump` branch exists, it opens a pull request of its own:
  - it commits the moves onto those files as origin's default branch has them, moving each of its pins of a differing path from whatever commit it names there;
  - it pushes that commit to a new `upstream-pin-bump` branch;
  - it opens the branch's pull request into the default branch, and prints the pull request's URL.
- Where the branch exists, its pull request is still open, so it commits the moves onto the files as that branch has them and pushes the commit there, over any commit a person added and without forcing, and the pull request shows it.
  - A branch whose pins are at the tips already is left as it is.
  - Merging the pull request deletes the branch where the repository deletes merged branches, and deleting it by hand works as well, after which the next run opens a pull request afresh.

Every pin a run moves goes to that one branch, whichever upstream it is of, so pins meant to be proposed apart take a run each, narrowed by pathspec, with `--branch-suffix <suffix>` giving them `upstream-pin-bump-<suffix>` in place of `upstream-pin-bump`, such as `mise run upstream:pin-check -- --bump --branch-suffix lints Cargo.toml clippy.toml` beside a run for `mise.toml`.
The suffix only extends the task's own name, so that a branch of that name exists only while its pull request is open, which is what lets a run take finding it to mean that.

The job needs this, which `contents: write` and `pull-requests: write` give the job's own token:

```yaml
# .github/workflows/<workflow>.yml, in the consuming repository
env:
  MISE_NO_HOOKS: "1"

jobs:
  upstream-pin-check:
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
    steps:
      - uses: actions/checkout@<sha>
      - uses: jdx/mise-action@<sha>
      - run: mise run upstream:pin-check -- --bump
        env:
          GITHUB_TOKEN: ${{ github.token }}
```

- It pushes through `origin` as the checkout left it, so the checkout has to keep its credentials, as `actions/checkout` does unless told `persist-credentials: false`.
- It opens the pull request through GitHub's REST API with curl, which [GitHub's hosted runner images carry](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md), on the token the step's `GITHUB_TOKEN` hands it.
  - A step sees the job's token only where its `env` names it, as [GitHub's own steps that call the API](https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/using-github-cli-in-workflows) do.
- GitHub lets the job's token open a pull request only where [the repository setting "Allow GitHub Actions to create and approve pull requests"](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/managing-github-actions-settings-for-a-repository#preventing-github-actions-from-creating-or-approving-pull-requests) is on, under "Workflow permissions" in Settings → Actions → General.
  - The same setting lets the token approve a pull request, and no permission separates the two.
- [GitHub starts no workflow run on a pull request the job's token opened or pushed to](https://docs.github.com/en/actions/writing-workflows/choosing-when-your-workflow-runs/triggering-a-workflow#triggering-a-workflow-from-a-workflow) until a person with write access selects "Approve workflows to run" on it, so its required checks wait for that.

Taking in what the upstream changed, such as running the shared tasks the move changes or bringing a lint over, stays with whoever merges that pull request.
With `--draft` as well, it opens the pull request as a draft, which GitHub [will not merge until someone marks it ready for review](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/about-pull-requests#draft-pull-requests).
That suits a pin whose path the repository took in part by hand, such as some of an upstream's lints copied into `Cargo.toml`, where moving the pin alone leaves the copy behind and merging the pull request as it stands would hide that: the person who brings the change over commits it to the pull request's branch and then marks it ready.
Only the pull request a run opens is a draft: a later run that finds the branch pushes its move onto it and leaves the pull request as it is, so a newer move can land on one already marked ready, and its change wants bringing over too before the merge.
Give such pins a `--branch-suffix` of their own, so that a move a merge alone finishes, such as an include's, does not wait behind them in the draft.
GitHub opens drafts in public repositories on every plan but in private ones only on GitHub Team and GitHub Enterprise Cloud, as [the endpoint that opens it](https://docs.github.com/en/rest/pulls/pulls#create-a-pull-request) says, so elsewhere GitHub refuses the pull request and the run ends as below.

It exits 2, pushing nothing, where a new branch's pull request could not be opened: without `GITHUB_TOKEN` in the step's `env`, or without curl.
It exits 2 at the push where the token cannot push, a pull request from a fork's among them, and after it where the pull request could not be opened: GitHub refused it, which prints GitHub's reason, or the API gave no answer within a minute.
It then deletes the branch it pushed, since a run that finds the branch takes its pull request to be open, so the next run opens one; where it cannot delete the branch it says so, and the branch has to be deleted by hand.
It exits 2 too on an option other than `--bump`, `--draft` and `--branch-suffix`, on `--draft` or `--branch-suffix` without `--bump`, on a suffix of anything but letters, digits, hyphens and underscores, and where the default branch holds none of the differing pins to move, a pin there whose path matches the tip's counting as none; a pull request cut before the last move merged can meet the second until it is brought up to date.
It exits 2 as well on an error from the filesystem, such as a scratch directory it could not create, or a file on the default branch it could not read.

## Writing one

Each task is an executable `*.sh`, and the directory it sits in is its namespace: `secrets/commit-scan.sh` is the task `secrets:commit-scan.sh`, which `mise run secrets:commit-scan` reaches as well.
Name it that way or the linting below never sees it, and mise will load it all the same.
mise [loads every executable under a task directory](https://mise.jdx.dev/tasks/file-tasks.html), at whatever depth, so an executable placed here to be run by another task would become a task of its own in every consuming repository, under a name nobody chose.
A file that is not executable is not loaded either, with one exception, and a task can source one through `$MISE_TASK_DIR`.
Nothing here does, and one named `*.sh` would be linted like a task, since that is what `mise.toml` selects on.
The exception is `.toml`: mise reads every one that is not a config file as a list of tasks, so a data file dropped in here becomes tasks named after its keys, or breaks task loading outright.
Keep such a file out of here.
[`task_config.excludes`](https://mise.jdx.dev/tasks/task-configuration.html#task_config.excludes) belongs to the config that declares the include, so naming it in this repository's `mise.toml` would clear it here and leave every consumer loading it.

A task here is a shell script, and its name says so: this repository hands every `*.sh` under this directory to shellcheck and `shfmt`, so a task in another language is named out of that and goes unchecked until the shell steps in `hk.pkl` reach it.
Where the work outgrows a few lines of shell, the script is a wrapper that runs a subcommand of the `dotfiles-tasks` binary, built from `rust/`.
The subcommand takes the task's name with its colon as a hyphen, as `link-check-staged` does for `link-check:staged`, and its module under `rust/src/` and its tests' under `rust/tests/tasks/` take that name with underscores.
It installs the release it pins with `mise install`, rather than declaring it in a [`#MISE tools=` header](https://mise.jdx.dev/tasks/task-configuration.html#tools), which installs nothing under `MISE_TASK_RUN_AUTO_INSTALL=false`, and runs the binary from the directory `mise where` prints, since [`mise x`](https://mise.jdx.dev/cli/exec.html) would leave the tools the header declares off `PATH`.
That install runs under `MISE_NO_HOOKS=1`, since mise runs the consuming project's [`[hooks] postinstall`](https://mise.jdx.dev/hooks.html) on every install there.
The line that spells the release ends in `# x-release-please-version`, and `release-please-config.json` lists the wrapper among its `extra-files`, so the release pull request moves its version.
A change under `rust/` reaches a consumer only once that pull request merges and the release is published, which `.github/workflows/release.yml` describes.
Declare in the task's `#MISE` header whatever tool mise can install for it, and start no later comment line with `MISE`.
mise reads the comment block following the header as more of the header, so a line opening with `MISE_TASK_DIR` or `MISE_CACHE_DIR` is parsed as a usage spec and every run of the task prints a parse warning.
Writing the variable with its sigil, as `$MISE_TASK_DIR`, keeps the line out of that.
Neither mistake shows up in this repository's own commit hook: it runs some of these tasks, `[tools]` in `mise.toml` puts their tools on PATH whatever their headers say, and the usage-spec warning appears only when the task it belongs to runs.

A task runs in the consuming repository: its working directory is that repository's root, and the paths it names resolve there rather than here.
[`MISE_TASK_DIR`](https://mise.jdx.dev/tasks/running-tasks.html#task-environment) is the one path that points back into this library — into the working tree here, or into a cached clone of this repository in a consumer.
It is the task's own namespace directory rather than the root of the library: `tasks/secrets` for `secrets:commit-scan`, so a file two tasks share sits at `$MISE_TASK_DIR/..`.

What a task must do is checked here without the consuming repositories ever running the check, by the `dotfiles-tasks` package's tests, which `qa:rust:crates` runs: each sets up a repository, runs the subcommand there, and checks the exit status and output, and a task's tests run its wrapper too, with a stand-in for mise.
They sit under `rust/tests/tasks/`, a module for each task; the one for `secrets:commit-scan`, which stays a shell script, runs the script itself.
A module of their own checks that each wrapper pins the release `rust/Cargo.toml` is at, since release-please skips a file its config names but cannot find without saying so, and a wrapper moved or renamed would go on pinning an old release.

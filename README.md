# dotfiles

This repository sets up a Mac: its shell, its tools, and the agent configuration every project on it loads.
[Using from Other Repositories](#using-from-other-repositories) covers what another repository takes from it.

## Setup

Homebrew is left to the user: install [Homebrew](https://brew.sh), and have it install what the `Brewfile` lists, mise and gh among them, from a clone of this repository.
Sign gh in before mise runs: mise installs most tools from GitHub, whose API limits requests without a token, and `MISE_GITHUB_CREDENTIAL_COMMAND` hands mise gh's token.

```shell
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
brew bundle
gh auth login
```

mise sets up the rest.
[mise refuses to parse a `mise.toml` from a directory it has not been told to trust](https://mise.jdx.dev/cli/trust.html), so trust this one first:

```shell
mise trust
MISE_GITHUB_CREDENTIAL_COMMAND="$(brew --prefix)/bin/gh auth token" mise bootstrap
```

The first run sets that variable by hand, since `.zshenv` exports it only once this run has had it source the shell fragments.
Both name gh by absolute path, for the reason `zsh_fragments/zshenv.sh` gives beside its export.

On a Mac already set up, rerun `brew bundle` before `mise bootstrap`, which installs nothing the `Brewfile` lists, gh included.

[`mise bootstrap`](https://mise.jdx.dev/bootstrap.html) applies what the root `mise.toml` declares, and a rerun changes only what has drifted:

- `[dotfiles]` symlinks the global mise config and the agent instructions below into place, and writes `~/.local/bin/notify_desktop` out from its `content`, over whatever is there.
  - It copies `.config/git/ignore` to `~/.config/git/ignore`, over whatever is there.
  - It sets cargo's [release window](#release-window) in `~/.cargo/config.toml` from `.config/cargo/config.toml`, leaving the file's other keys as they are.
  - It refuses to replace a file or directory already at a link's path, and changes nothing until that one is moved aside.
  - Each link points into the checkout it runs from, so a hook stops a run from a worktree before anything is written.
- It installs the tools that the global config and the root `mise.toml` pin.
- The `bootstrap` task runs last, on every run. It starts with `local:install`, which installs `dotfiles-local` into `~/.local/bin` from `main`, and fails on any other branch, stopping the steps after it. Then come `dotfiles:setup`, which runs that binary, `apm install`, and the `basic-memory` MCP server, as the sections below describe.

The global config, `.config/mise/config.toml`, holds the tools and settings that reach every repository; the root `mise.toml` pins the tools this repository's own tasks use.

A few more steps stay by hand, since each needs a secret, a sign-in or a path only this machine knows: rclone's remotes below, fnox keys under [Secrets](#secrets), and [K-Boat](#setting-up-k-boat)'s project registration and checkout path.

## Shell and User Config

`mise bootstrap` runs `mise run dotfiles:setup`, which does the following:

- Drops `~/.dircolors`, overwritten with canonical content.
- Makes `~/.zshenv`, `~/.zprofile` and `~/.zshrc` source this repository's shell fragments.
- Sets npm's [release window](#release-window) in `~/.npmrc`.
- Updates Hex and sets its [release window](#release-window) in `~/.hex/hex.config`.
- Encrypts rclone's config with a password it keeps in the login keychain.

### Shell startup: `.zshenv`, `.zprofile` and `.zshrc`

Each of the three files sources the fragment of the same name in `zsh_fragments/` (`zshenv.sh`, `zprofile.sh`, `zshrc.sh`); the `.zshenv` and `.zprofile` fragments say, line by line, why each line is in that file.
`dotfiles:setup` appends the sourcing line, pointing at the main checkout, to a file that does not already name its fragment, so an edit to a fragment reaches the next shell without rerunning setup, and lines an installer appends to the files stay.

#### `.zprofile`

`.zprofile` is read by login shells, and only after macOS's `/etc/zprofile` has run `/usr/libexec/path_helper` — so PATH set anywhere earlier is already demoted by then.
See [Homebrew discussion #1127](https://github.com/orgs/Homebrew/discussions/1127).

#### `.zshenv`

`.zshenv` is read by every shell, which is what `HOMEBREW_PREFIX` and the `nomatch` guard need.

- It also exports `RCLONE_PASSWORD_COMMAND`, which reads rclone's config password from the login keychain, so a config encrypted with it opens without a prompt in any shell, an agent's included.
- rclone runs the command only for an encrypted config, so an unencrypted one is unaffected.
- `dotfiles:setup` stores a random password where the command reads it, and [encrypts the config](https://rclone.org/docs/#configuration-encryption) with it, before any remote exists if need be; a remote added later with [`rclone config`](https://rclone.org/drive/) is saved encrypted.
- A config already encrypted with another password stops opening under the export, since a failing [password command](https://rclone.org/docs/#password-command) does not fall back to the prompt, and `dotfiles:setup` stops on it.
  - Decrypt it first with `env -u RCLONE_PASSWORD_COMMAND rclone config encryption remove`, which asks for that password; for a config copied from another Mac, the password command prints it there.
  - Without the password, as when the keychain item is gone, move the config aside and add the remotes again.
- It also defines the `uvx` and `uv` shell functions that give uv the [release window](#release-window).
- pitchfork daemons get the export too: launchd starts the supervisor with no shell environment, so pitchfork's `general.shell` has to be `/bin/zsh -c`, whose non-interactive zsh still reads `.zshenv`. Under the default `sh -c`, a daemon that runs rclone stalls on the password prompt and fails.

### Release window

Each package manager here waits a day before picking a newly published version, within the limits each item below names, so a compromised release pulled within hours is not taken in.
A day is the default [mise](https://mise.jdx.dev/configuration/settings.html#minimum_release_age) and pnpm 11 already apply, and the others are set to match it.

- The window applies when a version is picked, for a one-off install or a lockfile update, not to a version a lockfile already pins.
- npm, Hex and cargo take it from their user or global config rather than an environment variable, which would outrank a project's own config, so a project can still set its own.
- npm: `dotfiles:setup` sets [`min-release-age`](https://docs.npmjs.com/cli/v11/using-npm/config/) in `~/.npmrc`.
- Hex: `dotfiles:setup` sets [`cooldown`](https://hex.hexdocs.pm/Mix.Tasks.Hex.Config.html) in `~/.hex/hex.config`, which any `mix` reads, whatever shell starts it.
  - It covers a project's `mix.lock` when a dependency is added or updated, and `Mix.install` in a script, which has no lockfile and resolves again whenever its cache is gone, as after an Elixir or Erlang upgrade.
  - `dotfiles:setup` updates Hex first with `mix local.hex --force`: the Hex archive is built for one Erlang/OTP, so an older one can fail to load after an upgrade.
  - It updates two archives, the one under the Elixir install that mise's `MIX_HOME` names and the one in `~/.mix`, which a shell with `mix` on PATH but without mise's environment uses; both read the one `~/.hex/hex.config`.
- cargo: `[dotfiles]` merges [`global-min-publish-age`](https://doc.rust-lang.org/cargo/reference/config.html#registryglobal-min-publish-age) into `~/.cargo/config.toml`.
  - It [applies when cargo resolves a project's dependencies](https://doc.rust-lang.org/cargo/reference/resolver.html#publish-age), as `cargo update`, `cargo add` and `cargo generate-lockfile` do.
  - `cargo install` ignores it for the crate it installs, taking that crate's newest release whatever the key says.
  - Only cargo 1.100 and later enforce it; an older one does not, and only some releases warn that they ignore the key.
- uv: [`exclude-newer`](https://docs.astral.sh/uv/reference/settings/#exclude-newer) covers only `uvx` and `uv tool`, through shell functions in `.zshenv`: uv writes a user-wide window into each project's `uv.lock`, which then fails `uv lock --check` for anyone locking without it ([astral-sh/uv#18775](https://github.com/astral-sh/uv/issues/18775)).
  - A project gets the window by setting `[tool.uv] exclude-newer = "1 day"` in its own `pyproject.toml`, which every checkout then shares.
  - Other uv commands get none, including a one-off `uv run --with <pkg>` and a script's inline dependencies, so run a one-off through `uvx --with <pkg>` instead.
- APM has none of its own, so [`skills:update`](#agent-skills) holds back a skill changed under a day ago.
- To take a fix released within the day, override the window for that command.
  - npm: `npm install --min-release-age=0`.
  - uv: `UV_EXCLUDE_NEWER=false uvx …`.
  - cargo: `CARGO_REGISTRY_GLOBAL_MIN_PUBLISH_AGE="0 seconds" cargo update -p <crate>`.
  - Hex, in a project: `HEX_COOLDOWN=0d mix deps.update <dep>`.
  - Hex, in a `Mix.install` script: `HEX_COOLDOWN=0d MIX_INSTALL_FORCE=1 elixir <script>`; without `MIX_INSTALL_FORCE` the script loads what it cached and resolves nothing.

### Where a script goes

For a script shared across this machine, what it is for decides whether it has to be shell, whether its branches rejoin decides whether it is written in Rust, and who calls it decides where the rest goes.
Take the first of these that fits:

1. It changes the calling shell's state, such as its environment, `PATH`, working directory, or the settings of the shell or the terminal: shell, as a function or a line in a shell fragment, since a separate process cannot.
2. It runs before `dotfiles-local` can, as `local:install`, the `postinstall` hook in `[hooks]` with the `hook.*` commands it registers, and `[bootstrap.hooks]` do: shell, since the binary may not be there yet.
3. It holds a conditional whose paths rejoin, where a path that ran something only on some condition flows on into code written after the conditional: a subcommand of `dotfiles-local`, which its caller runs as `~/.local/bin/dotfiles-local <command>`.
   - A loop counts, since every pass returns to the same code.
   - A conditional whose every path ends with nothing after it does not count.
     - That holds however a path ends: succeeding, failing, printing a message or doing nothing.
     - It holds however the conditional is spelled: with `if`, `&&`, `||` or `case`, leaving by `return`, `exit` or the last line.
   - Variables and pipes do not count, since they split no path.
   - A name a person types is an alias or function in `zsh_fragments/zshrc.sh` that runs the subcommand.
   - A command a skill has an agent run is a subcommand too, which the skill names by that path, since an agent's shell does not read `.zshrc`.
4. Anything else stays shell, where its caller finds it:
   - a mise task's `run` in `mise.toml`, for one run as a mise task;
   - an alias or function in `zsh_fragments/zshrc.sh`, for one a person types;
   - a script written out as `content` in `[dotfiles]`, with `permissions = "755"`, for one another caller runs by its path in `~/.local/bin`.

A command that runs unattended, or whose caller does not start zsh, is one in `~/.local/bin`, a subcommand or a script written out as `content`, rather than a mise task.

- `mise run` loads the config of the directory it runs in and installs any missing tool before any task starts, and a task's own definition can turn off neither.
- So a `mise.toml` that cannot be parsed, a `git::` include that cannot be fetched, or a missing tool while offline fails a mise task, and a task of the same name in that directory runs in its place.
- `mise -C ~ run --skip-tools` avoids those only for as long as every caller remembers it, and a caller that forgets works until one of them happens, when nobody is watching.
- A caller that does not start zsh gives the command's full path, since `.zprofile` is what puts `~/.local/bin` on `PATH`.
- `qa:sh` checks only `.sh` files, so it does not reach a script written out as `content`, and an edit to one reaches `~/.local/bin` only at the next `mise dot apply` or `mise bootstrap`.

The shared tasks under `tasks/`, which other repositories run, follow [their own rules](tasks/README.md#writing-one) instead.
So do Claude Code's PreToolUse hooks: each goes in the `global-hooks` module as a `tool.check` hook with TypeScript tests, unless it rewrites the call's input, as [Agent Skills](#agent-skills) describes.

A command for this machine alone that is written in Rust is a subcommand of `dotfiles-local`, the binary of the `rust/crates/local` package, so that it is compiled with warnings as errors and tested in CI, a dependency it takes is pinned in `rust/Cargo.lock`, and every such command shares one name on `PATH`.

- `local:install` installs the binary into `~/.local/bin` from a checkout on `main`, and fails on any other branch, since every caller runs the installed copy.
  - `mise bootstrap` runs it, and so do the post-merge and post-checkout hooks that `mise install` registers, once `main` moves or a switch moves `HEAD` onto it.
  - A feature branch tries its change in its own worktree with `cargo run --manifest-path rust/Cargo.toml -p dotfiles-local -- <command>`; the installed binary runs main's code until the branch merges.
- Each command is a module, `src/<name>.rs`, with a `Request` that clap reads the command's arguments into, and a variant of `src/main.rs`'s `Command` holding it.
  - The request's doc comments are the command's help, and clap lists the command under that variant's name, so the help and the list of commands both come from the definition.
  - Its `run` returns `Result<(), Failure>`, and `main` alone reports a failure, with its message where it has one, as the exit status.
- The binary runs in its caller's working directory, so a mise task that runs it sets no `dir` and the command acts on the root of the checkout the task ran in, and a command a shell function runs reads a relative path from where the function was called.
- Its tests sit in `tests/commands/<name>.rs` and run the built binary, with a fake shell script first on `PATH` for each external command, and `mise run qa:rust` runs them with the rest of the workspace.
  - A test that needs a real tool, such as gitleaks or lychee, has CI's `rust-qa` job install it through `install_args`, and one that only macOS has, such as `osascript` or `qlmanage`, has a fake answer in its place, since that job runs on Linux.
- A dependency it adds or updates changes `rust/Cargo.lock`, which `dotfiles-tasks`'s releases cover, so a `feat` or `fix` commit with that change opens a release pull request.

A property of the shared tasks under `tasks/`, such as each wrapper pinning the `dotfiles-tasks` release `rust/Cargo.toml` is at, is tested in `rust/tests/tasks/` beside the tests of the tasks themselves, rather than checked by a `qa:` task of its own.

### Naming tasks and subcommands

A mise task is named `<area>:<action>` or `<area>:<what it produces>`, as `skills:update` and `ts:types` are.

- A check or a formatter takes the kind of work as its area and its target after it, as `qa:md` and `fmt:md` do, so that each kind of work is one prefix: `pre-commit` runs `qa:**`, and CI runs the same `qa:` tasks ([CI baseline](.claude/rules/ci-baseline.md#checks)).
  - `fmt` is short, as in `cargo fmt`.
- `qa:` holds only the checks that `pre-commit` and CI both run and that use no network.
  - A shared task itself, such as `secrets:commit-scan`, and a check that uses the network, such as `links:check`, sit under an area of their own instead.
  - `qa:rust:skills` stays under `qa:`, though it runs a shared task's subcommand: it runs it from the working tree, to check this repository.
- `pre-commit` and `bootstrap` are names mise sets, and fall outside the rule.

A subcommand of `dotfiles-local` or `dotfiles-tasks` takes the name of the mise task, or of the alias or function a person types, that runs it, with each `:` as `-`.

- One that several tasks run takes the part their names share.
  - A shared task's subcommand takes the shared task's name, as `skills-size-check` does, though a task in `mise.toml` such as `qa:rust:skills` runs it from the working tree as well.
- A skill that has an agent run a subcommand uses the name settled this way, and does not set one.
- One that neither a task nor a person runs by name, such as a hook or a command only a skill runs, keeps the name it has.

One name predates the rule and does not follow it yet: the shared task `link-check:staged` puts its action where its area goes.
Renaming it means changing, together, the release each wrapper pins, the variable that skips it, and the `pre-commit` of every repository that runs it, so it keeps its name until they change with it.

## Agent Instructions

- `CLAUDE.md` — this repository's own project instructions, loaded only for sessions working inside it.
- `.claude/CLAUDE.md` — user-scoped principles (shareable), symlinked to `~/.claude/CLAUDE.md`.
- `.claude/rules/` — path-scoped rules, loaded when Claude works with files matching each rule's `paths`, symlinked to `~/.claude/rules`.

A session in a worktree of this repository also finds the worktree's own copies of both as project instructions, which `.claude/settings.json` excludes through `claudeMdExcludes`, so the main checkout's, reached through those links, are the ones that load there.

## Agent Skills

Most skills live under `.claude/skills/`, managed by [APM](https://github.com/microsoft/apm), co-located with the instructions and rules above.
GitHub Copilot [also reads `.claude/skills/`](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills), so this single directory serves both the primary Claude setup and Copilot as a secondary client.
`apm.yml` declares the packages and `apm.lock.yaml` pins the resolved commits and content hashes.

The `target: claude` field in `apm.yml` is deliberately the **singular** `target:` key, not the plural `targets:`.
`apm uninstall` reads only the singular field, so with `target:` set it honors the pin and touches `.claude/skills/` alone.
A plural `targets:` reads as unset, which makes uninstall auto-detect on-disk targets (`.github/`, `.cursor/`, …) and mirror skills into a stray `.agents/skills/`.

`mise bootstrap` symlinks `~/.claude/skills` to it.
That symlink carries user-global hooks.
`.claude/skills/global-hooks/` holds a `.claude-plugin/plugin.json`, so Claude Code loads it in place as the [skills-directory plugin](https://code.claude.com/docs/en/plugins/create#scaffold-a-plugin-that-loads-every-session) `global-hooks@skills-dir` in every project, and its `hooks/hooks.json` stays out of the machine-local `~/.claude/settings.json`.
An edit to it takes effect after `/reload-plugins` or a restart.
The same `hooks/hooks.json` names one function-hooks module under `modules`, `hooks/index.ts`, since it takes one per plugin; that module registers the others:

- `hooks/block-bash.ts` blocks a Bash call that would not do what it reads as, over any rule that would allow it, and tells Claude what to write instead; where the hook itself fails, it blocks the call naming the cause rather than letting it through unjudged.
  - `hooks/block-piped-git-push.ts` blocks a `git push` chained with `&&` after a piped git merge, rebase, pull or commit, which pushes even when that step failed.
  - `hooks/block-bsd-sed-inplace.ts` and `hooks/block-bsd-stat-format.ts` block BSD's `sed -i ''` and `stat -f <format>` while the tool on `PATH` is GNU's, which reads the empty suffix or the format as a file name, so the call fails.
- `hooks/compact-reminder.tsx` shows a band above the prompt once the context reaches 300k tokens, so a `/compact` with instructions comes before the automatic one, which has none; `idleCompaction` is off in `.config/claude/settings.json` for the same reason.
- `hooks/connector-ask.ts` lays a base under the permission rules: even in [auto mode, whose classifier](https://code.claude.com/docs/en/permission-modes#how-the-classifier-evaluates-actions) would otherwise settle the call with no prompt, a connector tool asks the user unless its server declares it a read, since it acts on outside services under the user's account; deny and allow rules sit on top of that base, and a tool added later is covered without anyone listing it.
- `hooks/reply-language.ts` adds the reply-language context in `context/reply-language.md` to every prompt at `prompt.submit` rather than `classic.UserPromptSubmit`, since [an organization's managed plugin can continue a classic event past the user tier](https://github.com/irisTa56/dotfiles/pull/421), where a module's hook on it never runs.
- `hooks/review-loop.ts` supports the `review-loop` skill by showing the round of the branch's unclosed record in the status line.
- `hooks/scheduled-chrome.ts` denies Claude in Chrome's tools in a run a scheduled task started, in its main conversation and in its subagents, until a person replies in that session; the built-in browser stays available.
- `hooks/subagent-model.ts` runs the built-in Explore on Sonnet where the spawn names no model, which `CLAUDE_CODE_SUBAGENT_MODEL` does not reach, and asks the user before a subagent is passed a model above Sonnet, leading that call's description with "Runs on <model>" since the desktop app's prompt shows the description and not the ask's reason.

Beside the module, `hooks/hooks.json` registers `dotfiles-local ask-email-in-request` as a PreToolUse command hook, which asks before a curl or wget call that carries the user's email address and heads the call's description, which the prompt shows, with that cause.
A module's hook can rewrite the description only in `tool.call`, apart from the ask in `tool.check`, and [auto mode refuses a call whose input a hook changed after it reviewed the call](https://github.com/irisTa56/dotfiles/pull/473), where a command hook's rewrite and ask arrive together.

It also registers `dotfiles-local reply-language-stop` as a Stop command hook: [a managed plugin can continue `classic.Stop` past the user tier too](https://github.com/irisTa56/dotfiles/pull/424), and a command hook in `hooks.json` still runs where the module's hook on that event does not.
It sends a final reply of the main conversation whose prose outside code and quotations runs past a couple of English sentences with no Japanese sentence in it back once to be rewritten in Japanese, since final replies have switched to English after a turn filled with English tool output; a reply in the middle of a turn is left to drift.
It keeps what one stop leaves for the next in a file per session under the system temporary directory.

The Delegation rule in `.claude/CLAUDE.md` has an agent pass a subagent no model unless it wants Haiku or one the user or a skill names, so the cap on the rest is `CLAUDE_CODE_SUBAGENT_MODEL=sonnet` from `.config/claude/settings.json` together with that Explore hook; without the variable, those subagents run on the session's model.
Plan, a fork, and an agent whose definition names its own model are outside that cap: Plan and a fork run on the session's model, and a definition's model outranks the variable.

The function-hooks API is early access and may change with a Claude Code update, so `mise run pre-commit` runs the modules' tests with `claude plugin test` and type-checks them with `claude plugin validate`; `mise run ts:types` writes the installed CLI's declarations for an editor to type-check them against; Biome lints and formats them (`mise run qa:ts`, `mise run fmt:ts`), in CI as well.

Restore pinned skills from `apm.lock.yaml`, which `mise bootstrap` also does:

```shell
apm install
```

Add a new skill package (`owner/repo` for a single-skill repo, `owner/repo/path/to/skill` for a monorepo entry):

```shell
apm install owner/repo
```

Remove installed packages (also strips them from `apm.yml` and `apm.lock.yaml`):

```shell
mise run skills:remove <package> [more...]
```

Audit deployed files against the lockfile, plus integrity and hidden-character checks:

```shell
apm audit
```

Show packages whose upstream advanced past the pinned ref, then update them:

```shell
apm outdated
mise run skills:update [package...]
```

APM has no release window, so the task gives skills the one the other package managers have here ([Release window](#release-window)).
It holds back a skill whose path last changed in a pull request merged under a day ago, going by GitHub's merge time since a commit's own date is whatever its author set.
It also holds one when GitHub lists no such pull request for its last commit, though the list can miss one, so that hold is for checking by hand.
`apm update <package>` followed by `apm install` takes a held skill anyway.
`executables.allow` being empty in `apm.yml` keeps a package's hooks out of `.claude/settings.json`, though a `.claude-plugin/plugin.json` still deploys and loads its folder as a plugin.

A skill only some repositories want is installed here all the same and turned off for every project: `.config/claude/settings.json` sets it to `"off"` under [`skillOverrides`](https://code.claude.com/docs/en/skills#override-skill-visibility-from-settings), which hides it from Claude and from the `/` menu, and a repository that wants it sets it to `"on"` in its own `.claude/settings.json`.
`rust-best-practices` is one, since its advice is for Rust code alone.

### Gist-sourced skills

Some skills are published as a single-file GitHub gist holding a `SKILL.md`.
APM installs one from the gist's git URL like any other package, and pins it in `apm.lock.yaml`.
It deploys under a directory named after the gist hash unless the dependency carries an `alias`, so each is declared in `apm.yml` in the object form:

```yaml
- git: https://gist.github.com/<owner>/<gist_id>.git
  alias: <name>
```

Then run `apm install`.

The gist is the only durable copy of a skill's content, and `apm install` restores the pinned copy over a local edit, so push the edit back to the gist first:

```shell
mise run skills:push japanese-tech-writing
```

This commits the edit on top of the pinned revision in a temporary clone of the gist and pushes it with git, using `gh`'s credential for that push alone.
A gist that has moved past the pin rejects the push as non-fast-forward, which keeps the edit from dropping that revision.
It then runs `apm update` on that skill, which moves the pin to the pushed commit, and fails if the pin lands anywhere else; until that `apm.lock.yaml` change reaches `main`, an `apm install` from `main` restores the old copy, and a push from it is refused.

### Repo-tracked skills

A few skills are written here rather than pulled from an upstream, and this repository is their only copy.
`.gitignore` excludes all of `.claude/skills/*`, which is what keeps APM output out of version control.
A hand-written skill therefore needs one line to unignore it:

```gitignore
!/.claude/skills/<name>/
```

Re-including the directory is enough — the exclusion above uses a single `*`, which does not cross `/`, so it never matched the contents in the first place.

## MCP Servers

The only stdio MCP server in use is `basic-memory`.
Claude Desktop's DXT extensions and remote connectors are managed in-app, not from this directory.

`.config/claude/settings.json` keeps tools that sessions seldom or never called out of every request, since each tool definition is re-read from the prompt cache on every API request.
Its `permissions.deny` names them bare or by an `mcp__<server>__*` glob, which [removes the tool from Claude's context](https://code.claude.com/docs/en/permissions) rather than only refusing the call.
A server the desktop app provides itself is the exception: a rule for its tools only refuses the calls, and what the server adds to the prompt stays.
`mise bootstrap` [replaces an array outright](https://mise.jdx.dev/dotfiles/edits.html) when it merges this file, so this list is the whole of `permissions.deny` in `~/.claude/settings.json`, and a deny rule added there by hand is gone after the next bootstrap unless it is added here.
Computer use is one such server, denied so that its calls are refused: a web UI is checked through the built-in browser, and a native app is left for the user to look at, which also rules out a GUI action Claude did not mean to take.
Whether the app offers computer use at all is an in-app capability, which this repository does not set.
Its `enabledPlugins` turns off `cowork-plugin-management`, a plugin synced from the claude.ai account, by the [`<name>@synced` switch](https://code.claude.com/docs/en/settings-reference#syncclaudeaiplugins); the same plugin can also load as `<name>@inline`, so that entry is off too.

`mise bootstrap` registers `basic-memory` in Claude Code's user scope when it is not there yet:

```shell
claude mcp add-json -s user basic-memory '{"command":"uvx","args":["basic-memory","mcp"]}'
```

Another client is wired by hand.

### Setting up K-Boat

[K-Boat](https://github.com/irisTa56/k-boat) is a skill package that reads sources through NotebookLM and matures them into a concept graph.
It owns the writing side; this repository only reads what it records, through two repo-tracked skills:

- `ask-kboat` answers a question from the concept notes and keeps what they say distinct from general knowledge.
- `find-kboat-repo` finds software for a stated need among the GitHub repositories K-Boat catalogues in the vault's `Repos/` folder.

K-Boat stores that graph as a [Basic Memory *project*](https://github.com/basicmachines-co/basic-memory) — a name bound to a directory of Markdown notes — and the `ask-kboat` skill requires that project to be registered.
Setting up the MCP server above is not enough on its own because project registration is per-machine local state that no clone carries.

```shell
basic-memory project add k-boat-knowledge <KBOAT_KNOWLEDGE_PATH>
basic-memory project list
```

`find-kboat-repo` reads the catalogue through K-Boat's own `kboat-note` command, so it needs a k-boat checkout set up as K-Boat's README describes, and the path to it as `KBOAT_REPO` in the `env` of `~/.claude/settings.json`.
That file is machine-local, and `mise bootstrap` merges `.config/claude/settings.json` into it [key by key](https://mise.jdx.dev/dotfiles/edits.html), so the key survives while no tracked file names it.

```json
"env": { "KBOAT_REPO": "<path to the k-boat checkout>" }
```

## Using from Other Repositories

The sections above set this Mac up; the two below are for work inside another repository.
[Secrets](#secrets) works only on a Mac set up from here, since its command is an alias the shell fragments define for a subcommand of `dotfiles-local`, while the [shared mise tasks](#shared-mise-tasks) come in through an include and run wherever mise does.

### Secrets

API keys stay out of every file a repository keeps, `.env` and mise's `[env]` included, so those hold only settings anyone may read, an agent included.
[fnox](https://fnox.jdx.dev), which the global mise config installs, keeps each key in the macOS login keychain and hands it to one command at a time.

- Store a key from anywhere in the repository, typing it at the prompt so it lands in neither shell history nor a process's arguments:

  ```shell
  fnox-add-secret <NAME>
  ```

  - The command declares a keychain provider named after the repository in a `fnox.local.toml` at the main checkout's root, which the global git ignore that `mise bootstrap` copies into place keeps out of every repository, and has fnox add the key's entry there.
  - It also creates the root's `CLAUDE.local.md`, likewise ignored, with a line telling an agent there to run what needs a key through fnox; if the file already exists without that line, the command prints it for you to place instead.
  - The login keychain does not sync through iCloud, so another Mac needs the key stored again.

- Run what needs the key as `fnox exec -- <command>`, which puts it in that command's environment alone; `fnox activate` would export it to everything run in the directory.

- A worktree has no `fnox.local.toml` of its own. Claude Code puts worktrees under the main checkout's `.claude/worktrees/`, where fnox finds the main checkout's by searching upward; a worktree placed elsewhere does not.

`.claude/rules/secrets.md` tells an agent the same when it opens `.env`, mise config or fnox config in any repository.

### Shared mise Tasks

`tasks/` holds the repository-agnostic tasks this repository lends to others:

- a gitleaks scan of a commit's staged changes;
- a networked check of the links in the files a commit stages;
- a trufflehog scan of the commits a push would send;
- a check that a consumer's pinned copy of these tasks is the current one.

This repository runs the first three itself, the same way a consumer would, from the `pre-commit` and `pre-push` hooks that `mise install` sets up.
[tasks/README.md](tasks/README.md) is where a repository taking them starts, and where the constraints on writing another are stated.

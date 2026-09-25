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
MISE_GITHUB_CREDENTIAL_COMMAND="gh auth token" mise bootstrap
```

The first run sets that variable by hand, since `.zshenv` exports it only once this run has had it source the shell fragments.

On a Mac already set up, rerun `brew bundle` before `mise bootstrap`, which installs nothing the `Brewfile` lists, gh included.

[`mise bootstrap`](https://mise.jdx.dev/bootstrap.html) applies what the root `mise.toml` declares, and a rerun changes only what has drifted:

- `[dotfiles]` symlinks the global mise config and file tasks, and the agent instructions below, into place, and adds their two lines to `~/.claude/CLAUDE.md`.
  - It refuses to replace a file or directory already at a link's path, and changes nothing until that one is moved aside.
  - Each link points into the checkout it runs from, so a hook stops a run from a worktree before anything is written.
- It installs the tools that the global config and the root `mise.toml` pin.
- The `bootstrap` task runs last, on every run, as the sections below describe: `setup:dotfiles`, `rtk init`, `apm install`, `skills:sync`, and the `basic-memory` MCP server.

The global config, `.config/mise/config.toml`, holds the tools, settings and tasks that reach every repository, and `.config/mise/tasks/` its file tasks; the root `mise.toml` pins the tools this repository's own tasks use.

A few more steps stay by hand, since each needs a secret, a sign-in or a path only this machine knows: rclone's remotes below, fnox keys under [Secrets](#secrets), and [K-Boat](#setting-up-k-boat)'s project registration.

## Shell and User Config

`mise bootstrap` runs `mise run setup:dotfiles`, which drops `~/.dircolors` and `~/.config/git/ignore` (each is overwritten with canonical content), makes `~/.zshenv`, `~/.zprofile` and `~/.zshrc` source this repository's shell fragments, sets pitchfork's `general.shell` in `~/.config/pitchfork/config.toml`, sets npm's `min-release-age` in `~/.npmrc`, and encrypts rclone's config with a password it keeps in the login keychain.

### Shell startup: `.zshenv`, `.zprofile` and `.zshrc`

Each of the three files sources the fragment of the same name in `zsh_fragments/` (`zshenv.sh`, `zprofile.sh`, `zshrc.sh`); the `.zshenv` and `.zprofile` fragments say, line by line, why each line is in that file.
`scripts/setup_dotfiles.sh` appends the sourcing line, pointing at the main checkout, to a file that does not already name its fragment, so an edit to a fragment reaches the next shell without rerunning setup, and lines an installer appends to the files stay.

#### `.zprofile`

`.zprofile` is read by login shells, and only after macOS's `/etc/zprofile` has run `/usr/libexec/path_helper` — so PATH set anywhere earlier is already demoted by then.
See [Homebrew discussion #1127](https://github.com/orgs/Homebrew/discussions/1127).

#### `.zshenv`

`.zshenv` is read by every shell, which is what `HOMEBREW_PREFIX` and the `nomatch` guard need.

- It also exports `RCLONE_PASSWORD_COMMAND`, which reads rclone's config password from the login keychain, so a config encrypted with it opens without a prompt in any shell, an agent's included.
- rclone runs the command only for an encrypted config, so an unencrypted one is unaffected.
- `setup:dotfiles` stores a random password where the command reads it, and [encrypts the config](https://rclone.org/docs/#configuration-encryption) with it, before any remote exists if need be; a remote added later with [`rclone config`](https://rclone.org/drive/) is saved encrypted.
- A config already encrypted with another password stops opening under the export, since a failing [password command](https://rclone.org/docs/#password-command) does not fall back to the prompt, and `setup:dotfiles` stops on it.
  - Decrypt it first with `env -u RCLONE_PASSWORD_COMMAND rclone config encryption remove`, which asks for that password; for a config copied from another Mac, the password command prints it there.
  - Without the password, as when the keychain item is gone, move the config aside and add the remotes again.
- It also makes uv ([`exclude-newer`](https://docs.astral.sh/uv/reference/settings/#exclude-newer)) pick no package version published less than a day ago, as the script makes npm do through its user config ([`min-release-age`](https://docs.npmjs.com/cli/v11/using-npm/config/)); mise and pnpm 11 already wait a day by default.
  - The window applies when a version is picked, for a one-off install or a lockfile update, not to a version a lockfile already pins.
  - npm's sits in the user config, below a project's own `.npmrc`, so a project can set a longer one.
  - For uv it covers only `uvx` and `uv tool`, through shell functions: uv writes a user-wide window into each project's `uv.lock`, which then fails `uv lock --check` for anyone locking without it ([astral-sh/uv#18775](https://github.com/astral-sh/uv/issues/18775)).
    - A project gets the window by setting `[tool.uv] exclude-newer = "1 day"` in its own `pyproject.toml`, which every checkout then shares.
    - Other uv commands get none, including a one-off `uv run --with <pkg>` and a script's inline dependencies, so run a one-off through `uvx --with <pkg>` instead.
  - To take a fix released within the day, override it for that command: `npm install --min-release-age=0`, or `UV_EXCLUDE_NEWER=false uvx …`.
- pitchfork daemons get the export too: launchd starts the supervisor with no shell environment, so the script sets pitchfork's `general.shell` to `/bin/zsh -c`, whose non-interactive zsh still reads `.zshenv`. Under the default `sh -c`, a daemon that runs rclone stalls on the password prompt and fails.

## Agent Instructions

- `CLAUDE.md` — this repository's own project instructions, loaded only for sessions working inside it.
- `.claude/INSTRUCTIONS.md` — user-scoped principles (shareable), symlinked to `~/.claude/INSTRUCTIONS.md`.
- `~/.claude/RTK.md` — private and machine-local, not managed here; `mise bootstrap` writes it through `rtk init`.
- `.claude/rules/` — path-scoped rules, loaded when Claude works with files matching each rule's `paths`.

`~/.claude/CLAUDE.md` is a thin, machine-local entry point that imports the user-scoped parts.
`mise bootstrap` links them, adds the `@INSTRUCTIONS.md` and `@RTK.md` lines to that file, leaving any other line in it, and runs `rtk init -g --auto-patch` for the [rtk hook](https://www.rtk-ai.app/) that `.claude/INSTRUCTIONS.md` assumes.

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
The rtk hook stays in `~/.claude/settings.json`, since `rtk init` writes it there.

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

Show packages whose upstream advanced past the pinned ref, then update:

```shell
apm outdated
apm install --update
```

### Gist-sourced skills

Some skills are published as a single-file GitHub gist, which APM deploys under a directory named after the gist hash rather than a readable name.
These are vendored from `gistSkills.json`, a `name -> raw gist URL` catalog, by `scripts/sync_gist_skills.sh`.
The catalog is the source of truth, and the materialized `.claude/skills/<name>/SKILL.md` is gitignored like APM deps.
Unlike APM packages, these are not restored by `apm install`; `mise bootstrap` runs `mise run skills:sync` after it, which overwrites every one with its gist's copy.

List the catalog, then sync every entry (or one by name):

```shell
mise run skills:list
mise run skills:sync
mise run skills:sync japanese-tech-writing
```

Add a skill by putting a `name -> raw gist URL` entry in `gistSkills.json`, then run `mise run skills:sync <name>`.

The gist is the only durable copy of a skill's content, so push a local edit back to it before the next sync:

```shell
mise run skills:push japanese-tech-writing
```

This goes through the GitHub API (`gh` auth required) and verifies the result, since `gh gist edit` silently no-ops in a non-interactive shell.

### Repo-tracked skills

A few skills are written here rather than pulled from an upstream, and this repository is their only copy.
`.gitignore` excludes all of `.claude/skills/*`, which is what keeps APM and gist output out of version control.
A hand-written skill therefore needs one line to unignore it:

```gitignore
!/.claude/skills/<name>/
```

Re-including the directory is enough — the exclusion above uses a single `*`, which does not cross `/`, so it never matched the contents in the first place.

## MCP Servers

The only stdio MCP server in use is `basic-memory`, already configured in Claude Desktop and Claude Code.
Claude Desktop's DXT extensions and remote connectors are managed in-app, not from this directory.

`mise bootstrap` registers `basic-memory` in Claude Code's user scope when it is not there yet:

```shell
claude mcp add-json -s user basic-memory '{"command":"uvx","args":["basic-memory","mcp"]}'
```

Another client is wired by hand.

### Setting up K-Boat

[K-Boat](https://github.com/irisTa56/k-boat) is a skill package that reads sources through NotebookLM and matures them into a concept graph.
It owns the writing side; this repository only reads that graph, through the repo-tracked `ask-kboat` skill, which answers a question from the concept notes and keeps what they say distinct from general knowledge.

K-Boat stores that graph as a [Basic Memory *project*](https://github.com/basicmachines-co/basic-memory) — a name bound to a directory of Markdown notes — and the `ask-kboat` skill requires that project to be registered.
Setting up the MCP server above is not enough on its own because project registration is per-machine local state that no clone carries.

```shell
basic-memory project add k-boat-knowledge <KBOAT_KNOWLEDGE_PATH>
basic-memory project list
```

## Using from Other Repositories

The sections above set this Mac up; the two below are for work inside another repository.
[Secrets](#secrets) works only on a Mac set up from here, since its task comes with the global mise config, while the [shared mise tasks](#shared-mise-tasks) come in through an include and run wherever mise does.

### Secrets

API keys stay out of every file a repository keeps, `.env` and mise's `[env]` included, so those hold only settings anyone may read, an agent included.
[fnox](https://fnox.jdx.dev), which the global mise config installs, keeps each key in the macOS login keychain and hands it to one command at a time.

- Store a key from anywhere in the repository, typing it at the prompt so it lands in neither shell history nor a process's arguments:

  ```shell
  mise run secrets:add <NAME>
  ```

  - The task declares a keychain provider named after the repository in a `fnox.local.toml` at the main checkout's root, which the global git ignore that `mise run setup:dotfiles` writes keeps out of every repository, and has fnox add the key's entry there.
  - It also creates the root's `CLAUDE.local.md`, likewise ignored, with a line telling an agent there to run what needs a key through fnox; if the file already exists without that line, the task prints it for you to place instead.
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

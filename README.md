# dotfiles

## Prerequisites

Homebrew is the one thing nothing here installs, and everything below needs it.
Install [Homebrew](https://brew.sh), and let it install the rest:

```shell
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
brew bundle
```

The above supplies mise, which supplies additional tools and linters.
The tools and settings that reach every repository live in this repository's `.config/mise/config.toml`, which mise reads once it is symlinked into place; the root `mise.toml` pins the tools this repository's own tasks use.
[mise refuses to parse a `mise.toml` from a directory it has not been told to trust](https://mise.jdx.dev/cli/trust.html), so trust this one before installing:

```shell
mkdir -p ~/.config/mise
ln -sf "$PWD/.config/mise/config.toml" ~/.config/mise/config.toml
mise trust
mise install
```

## Initial Setup

Run once on a new machine to drop `~/.dircolors` and `~/.config/git/ignore` (each is overwritten with canonical content), to make `~/.zshenv`, `~/.zprofile` and `~/.zshrc` source this repository's shell fragments, to set pitchfork's `general.shell` in `~/.config/pitchfork/config.toml`, and to set npm's `min-release-age` in `~/.npmrc`:

```shell
mise run setup:dotfiles
```

### Shell startup: `.zshenv`, `.zprofile` and `.zshrc`

Each of the three files sources the fragment of the same name in `zsh_fragments/` (`zshenv.sh`, `zprofile.sh`, `zshrc.sh`); the `.zshenv` and `.zprofile` fragments say, line by line, why each line is in that file.
`scripts/setup_dotfiles.sh` appends the sourcing line, pointing at the main checkout, to a file that does not already name its fragment, so an edit to a fragment reaches the next shell without rerunning setup, and lines an installer appends to the files stay.

- `.zprofile` is read by login shells, and only after macOS's `/etc/zprofile` has run `/usr/libexec/path_helper` — so PATH set anywhere earlier is already demoted by then. See [Homebrew discussion #1127](https://github.com/orgs/Homebrew/discussions/1127).
- `.zshenv` is read by every shell, which is what `HOMEBREW_PREFIX` and the `nomatch` guard need.
  - It also exports `RCLONE_PASSWORD_COMMAND`, which reads rclone's config password from the login keychain, so a config encrypted with it opens without a prompt in any shell, an agent's included.
  - rclone runs the command only for an encrypted config, so an unencrypted one is unaffected.
  - Set it up once per machine:

    ```shell
    security add-generic-password -a rclone -s config -w "$(openssl rand -base64 40)"
    rclone config encryption set --password-command "/usr/bin/security find-generic-password -a rclone -s config -w"
    ```

  - A config already encrypted with a typed password stops opening under the export, since a failing password command does not fall back to the prompt; decrypt it first with `env -u RCLONE_PASSWORD_COMMAND rclone config encryption remove`, which asks for that password.
  - It also makes uv ([`exclude-newer`](https://docs.astral.sh/uv/reference/settings/#exclude-newer)) pick no package version published less than a day ago, as the script makes npm do through its user config ([`min-release-age`](https://docs.npmjs.com/cli/v11/using-npm/config/)); mise and pnpm 11 already wait a day by default.
    - The window applies when a version is picked, for a one-off install or a lockfile update, not to a version a lockfile already pins.
    - npm's sits in the user config, below a project's own `.npmrc`, so a project can set a longer one.
    - For uv it covers only `uvx` and `uv tool`, through shell functions: uv writes a user-wide window into each project's `uv.lock`, which then fails `uv lock --check` for anyone locking without it ([astral-sh/uv#18775](https://github.com/astral-sh/uv/issues/18775)).
      - A project gets the window by setting `[tool.uv] exclude-newer = "1 day"` in its own `pyproject.toml`, which every checkout then shares.
      - Other uv commands get none, including a one-off `uv run --with <pkg>` and a script's inline dependencies, so run a one-off through `uvx --with <pkg>` instead.
    - To take a fix released within the day, override it for that command: `npm install --min-release-age=0`, or `UV_EXCLUDE_NEWER=false uvx …`.
  - pitchfork daemons get the export too: launchd starts the supervisor with no shell environment, so the script sets pitchfork's `general.shell` to `/bin/zsh -c`, whose non-interactive zsh still reads `.zshenv`. Under the default `sh -c`, a daemon that runs rclone stalls on the password prompt and fails.

## Shared mise Tasks

`tasks/` holds the repository-agnostic tasks this repository lends to others: a gitleaks scan of a commit's staged changes, a trufflehog scan of the commits a push would send, and a check that a consumer's pinned copy of these tasks is the current one.
This repository runs the two scans itself, the same way a consumer would, from the `pre-commit` and `pre-push` hooks that `mise install` sets up.
[tasks/README.md](tasks/README.md) is where a repository taking them starts, and where the constraints on writing another are stated.

## Agent Instructions

- `CLAUDE.md` — this repository's own project instructions, loaded only for sessions working inside it.
- `.claude/INSTRUCTIONS.md` — user-scoped principles (shareable), symlinked to `~/.claude/INSTRUCTIONS.md`.
- `~/.claude/RTK.md` — private and machine-local, not managed here; the block below writes it.
- `.claude/rules/` — path-scoped rules, loaded when Claude works with files matching each rule's `paths`.

`~/.claude/CLAUDE.md` is a thin, machine-local entry point that imports the user-scoped parts.
Wire them, and the [rtk hook](https://www.rtk-ai.app/) that `.claude/INSTRUCTIONS.md` assumes, once on a new machine:

```shell
ln -sf "$PWD/.claude/INSTRUCTIONS.md" ~/.claude/INSTRUCTIONS.md
ln -sfn "$PWD/.claude/rules" ~/.claude/rules
cat >~/.claude/CLAUDE.md <<'EOF'
@INSTRUCTIONS.md
@RTK.md
EOF
rtk init -g --auto-patch
```

## Agent Skills

Most skills live under `.claude/skills/`, managed by [APM](https://github.com/microsoft/apm), co-located with the instructions and rules above.
GitHub Copilot [also reads `.claude/skills/`](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills), so this single directory serves both the primary Claude setup and Copilot as a secondary client.
`apm.yml` declares the packages and `apm.lock.yaml` pins the resolved commits and content hashes.

The `target: claude` field in `apm.yml` is deliberately the **singular** `target:` key, not the plural `targets:`.
`apm uninstall` reads only the singular field, so with `target:` set it honors the pin and touches `.claude/skills/` alone.
A plural `targets:` reads as unset, which makes uninstall auto-detect on-disk targets (`.github/`, `.cursor/`, …) and mirror skills into a stray `.agents/skills/`.

Symlink `~/.claude/skills` to it once:

```shell
ln -sfn "$PWD/.claude/skills" ~/.claude/skills
```

The same symlink carries user-global hooks.
`.claude/skills/global-hooks/` holds a `.claude-plugin/plugin.json`, so Claude Code loads it in place as the [skills-directory plugin](https://code.claude.com/docs/en/plugins-reference#skills-directory-plugins) `global-hooks@skills-dir` in every project, and its `hooks/hooks.json` stays out of the machine-local `~/.claude/settings.json`.
An edit to it takes effect after `/reload-plugins` or a restart.
The rtk hook stays in `~/.claude/settings.json`, since `rtk init` writes it there.

Restore pinned skills from `apm.lock.yaml`:

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
Unlike APM packages, these are not restored by `apm install`; run `mise run skills:sync` separately.

List the catalog, then sync every entry (or one by name):

```shell
mise run skills:list
mise run skills:sync
mise run skills:sync japanese-tech-writing
```

Add a skill by putting a `name -> raw gist URL` entry in `gistSkills.json`, then run `mise run skills:sync <name>`.

The gist is the only durable copy of a skill's content, so push a local edit back to it:

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

To wire `basic-memory` into a fresh client:

```shell
claude mcp add-json -s user basic-memory '{"command":"uvx","args":["basic-memory","mcp"]}'
```

### Setting up K-Boat

[K-Boat](https://github.com/irisTa56/k-boat) is a skill package that reads sources through NotebookLM and matures them into a concept graph.
It owns the writing side; this repository only reads that graph, through the repo-tracked `ask-kboat` skill, which answers a question from the concept notes and keeps what they say distinct from general knowledge.

K-Boat stores that graph as a [Basic Memory *project*](https://github.com/basicmachines-co/basic-memory) — a name bound to a directory of Markdown notes — and the `ask-kboat` skill requires that project to be registered.
Setting up the MCP server above is not enough on its own because project registration is per-machine local state that no clone carries.

```shell
basic-memory project add k-boat-knowledge <KBOAT_KNOWLEDGE_PATH>
basic-memory project list
```

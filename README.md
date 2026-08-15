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

Run once on a new machine to drop `~/.dircolors`, `~/.config/git/ignore`, `~/.zprofile`, and `~/.zshenv` (each is overwritten with canonical content):

```shell
mise run setup:dotfiles
```

### Trusting agent worktrees

mise trusts a config file by path, and [it shares that trust with a repository's linked worktrees](https://mise.jdx.dev/cli/trust.html), so an ordinary repository needs nothing here.
That sharing does not reach this one.
mise resolves a linked worktree to its main checkout through the shared git directory, which it expects to be a `.git` sitting in that checkout; `dotfiles` is a submodule of `my-foam`, so its shared git directory is `my-foam/.git/modules/dotfiles`, which is no checkout's `.git`.
A fresh worktree under `.claude/worktrees/` is therefore an untrusted path where `mise run pre-commit` refuses to run until someone trusts it by hand.
[`trusted_config_paths`](https://mise.jdx.dev/configuration/settings.html#trusted_config_paths) trusts everything under the directories it lists, which covers each new worktree as it appears.

Its entries are absolute paths, and trusting one is a decision about this machine rather than about the repository, so they belong in `~/.config/mise/config.local.toml`, [a user-local override](https://mise.jdx.dev/configuration.html) no repository tracks.
Write it on a new machine, from the clone:

```shell
cat >~/.config/mise/config.local.toml <<EOF
# Machine-local mise settings; see the dotfiles README ("Trusting agent worktrees").
[settings]
trusted_config_paths = ["$PWD/.claude/worktrees"]
EOF
```

Add a directory only where mise's own sharing does not reach it, which `mise trust --show` from a fresh worktree tells you, and never a repository root or a directory a new clone lands in.
What is granted here is granted to every config file below, including a branch's own `mise.toml`, whose `[hooks]` mise then runs unprompted, and a `mise.local.toml` that `~/.config/git/ignore` keeps out of every diff.

### Shell startup: `.zprofile` vs `.zshenv`

`scripts/setup_dotfiles.sh` writes both files and each line there says why it is where it is. What decides the split is which shells read which file:

- `.zprofile` is read by login shells, and only after macOS's `/etc/zprofile` has run `/usr/libexec/path_helper` — so PATH set anywhere earlier is already demoted by then.
- `.zshenv` is read by all of them, which is what `HOMEBREW_PREFIX` needs: an interactive shell that is not a login shell reaches `zshrc_fragment.sh` without having read `.zprofile`, and the `ls` alias there fails with `exit 127` when the variable is unset.

## Agent Instructions

- `CLAUDE.md` — this repository's own project instructions, loaded only for sessions working inside it.
- `.claude/INSTRUCTIONS.md` — user-scoped principles (shareable), symlinked to `~/.claude/INSTRUCTIONS.md`.
- `~/.claude/RTK.md` — private and machine-local, not managed here (create it separately).
- `.claude/rules/` — path-scoped rules, loaded when Claude works with files matching each rule's `paths`.

`~/.claude/CLAUDE.md` is a thin, machine-local entry point that imports the user-scoped parts. Wire them once on a new machine:

```shell
ln -sf "$PWD/.claude/INSTRUCTIONS.md" ~/.claude/INSTRUCTIONS.md
ln -sfn "$PWD/.claude/rules" ~/.claude/rules
cat >~/.claude/CLAUDE.md <<'EOF'
@INSTRUCTIONS.md
@RTK.md
EOF
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

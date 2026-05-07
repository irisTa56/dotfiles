# dotfiles

## Initial Setup

Run once on a new machine to drop `~/.dircolors`, `~/.config/git/ignore`, and `~/.zprofile`:

```shell
mise run setup:dotfiles
```

## Tool Versions

Upgrade tools to the newest version released on or before a given date:

```shell
mise upgrade --before 7d --dry-run
mise upgrade --before 7d
mise upgrade --before 2024-06-01
```

## Agent Skills

Skills live under `.claude/skills/`, managed by [APM](https://github.com/microsoft/apm).
`apm.yml` declares the packages and `apm.lock.yaml` pins the resolved commits and content hashes.
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
apm uninstall <package> [more...]
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

## MCP Servers

Claude Desktop is treated as the source of truth.
`mcpServers.json` is a snapshot dumped from its config; per-client installs read from this snapshot.

Refresh the snapshot after editing Claude Desktop:

```shell
mise run mcp:dump
```

See which servers are missing or extra in each client relative to the snapshot:

```shell
mise run mcp:diff
```

Install one server into Claude Code (user scope) or VS Code (user profile) from this directory:

```shell
mise run mcp:claude basic-memory
mise run mcp:vscode basic-memory
```

Environment variable placeholders in `mcpServers.json` are expanded via `envsubst` at install time.

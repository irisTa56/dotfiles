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

Skills live under `.agents/skills/`, which [Skills CLI](https://github.com/vercel-labs/skills) writes to.
Symlink `~/.claude/skills` to it once:

```shell
ln -sfn "$PWD/.agents/skills" ~/.claude/skills
```

Restore pinned skills from `skills-lock.json`:

```shell
skills experimental_install
```

Add a new skill (drops the redundant `.claude/skills/` and `.continue/skills/` symlink dirs that the CLI creates):

```shell
mise run skills:add owner/repo skill-name
```

Remove installed skills (same cleanup, also prunes `skills-lock.json`):

```shell
mise run skills:remove skill-name [more-skills...]
```

Diff installed skills against the lock file (`<` lines are missing, `>` lines are unpinned drift):

```shell
mise run skills:diff
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

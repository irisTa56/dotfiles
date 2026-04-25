# dotfiles

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

Diff installed skills against the lock file (`<` lines are missing, `>` lines are unpinned drift):

```shell
mise run skills-diff
```

## MCP Servers

```json
{
  "mcpServers": {
    "basic-memory": {
      "command": "uvx",
      "args": ["basic-memory", "mcp"]
    },
    "colab-mcp": {
      "command": "uvx",
      "args": ["git+https://github.com/googlecolab/colab-mcp"],
      "timeout": 30000
    }
  }
}
```

## Development

Set up pre-commit hooks with [mise](https://mise.jdx.dev/cli/generate/git-pre-commit.html).

```shell
mise generate git-pre-commit --write --task=pre-commit
```

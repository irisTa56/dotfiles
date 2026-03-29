# dotfiles

## Agent Skills

Check installed skills:

```shell
.claude/check_installed_skills.sh
```

Install from GitHub repositories via [Skills CLI](https://github.com/vercel-labs/skills):

```shell
xargs -L1 npx skills add --agent claude-code --yes < .claude/skills.txt
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

## Setup

Set up pre-commit hooks with [mise](https://mise.jdx.dev/cli/generate/git-pre-commit.html).

```shell
mise generate git-pre-commit --write --task=pre-commit
```

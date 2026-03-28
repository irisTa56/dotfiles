# Claude

## Agent Skills

- <https://agentskills.io/home>
  - Standard specification and guidelines.
- <https://github.com/anthropics/skills>
  - Anthropic's implementation and examples.
- <https://github.com/github/awesome-copilot/blob/main/docs/README.skills.md>
  - GitHub Copilot's curated list of skills.
- <https://github.com/VoltAgent/awesome-agent-skills>
  - VoltAgent's curated list of skills.

### Check installed skills

```shell
.claude/check_installed_skills.sh
```

### Install from [Smithery Registry](https://smithery.ai/skills) via [Smithery CLI](https://github.com/smithery-ai/cli)

Skill list: [smithery-skills.txt](smithery-skills.txt)

```shell
xargs -I{} smithery skill add --agent claude-code {} < .claude/smithery-skills.txt
```

### Install from GitHub repositories via [SkillPort](https://github.com/gotalab/skillport)

Skill list: [skillport-skills.txt](skillport-skills.txt)

```shell
xargs -L1 skillport --skills-dir .claude/skills add -y < .claude/skillport-skills.txt
```

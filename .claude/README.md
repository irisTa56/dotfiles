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

### Install from [Smithery Registry](https://smithery.ai/skills) via [Smithery CLI](https://github.com/smithery-ai/cli)

```shell
cat <<'EOF' | xargs -I{} smithery skill add --agent claude-code {}
davila7/humanizer
k-dense-ai/citation-management
k-dense-ai/dask
k-dense-ai/exploratory-data-analysis
k-dense-ai/geomaster
k-dense-ai/geopandas
k-dense-ai/hypothesis-generation
k-dense-ai/literature-review
k-dense-ai/markdown-mermaid-writing
k-dense-ai/markitdown
k-dense-ai/matplotlib
k-dense-ai/networkx
k-dense-ai/open-notebook
k-dense-ai/plotly
k-dense-ai/polars
k-dense-ai/scientific-brainstorming
k-dense-ai/scientific-critical-thinking
k-dense-ai/scientific-visualization
k-dense-ai/seaborn
k-dense-ai/statistical-analysis
EOF
```

### Install from GitHub repositories via [SkillPort](https://github.com/gotalab/skillport)

```shell
cat <<'EOF' | xargs -L1 skillport --skills-dir .claude/skills add -y
anthropics/skills skills/skill-creator
github/awesome-copilot skills/agentic-eval
github/awesome-copilot skills/architecture-blueprint-generator
github/awesome-copilot skills/autoresearch
github/awesome-copilot skills/context-map
github/awesome-copilot skills/create-implementation-plan
github/awesome-copilot skills/create-readme
github/awesome-copilot skills/create-specification
github/awesome-copilot skills/create-technical-spike
github/awesome-copilot skills/documentation-writer
github/awesome-copilot skills/git-commit
github/awesome-copilot skills/prd
github/awesome-copilot skills/refactor
github/awesome-copilot skills/refactor-plan
github/awesome-copilot skills/update-implementation-plan
jgraph/drawio-mcp skill-cli/drawio
EOF
```

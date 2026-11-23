---
description: An English incident log about a failed deployment, to be summarized in Japanese, whose terms all have established katakana forms that none of the skill's examples name
max_turns: 8
allowed_tools: [Skill]
---

次の障害メモを、社内の障害報告として日本語の 1 段落（4〜6 文）にまとめてください。
報告の本文だけを返してください。

```text
- 14:02 Merged PR 812 into main; CI started the build.
- 14:09 The build passed, but it reused a stale cache from the previous commit, so the artifact still contained the old config.
- 14:15 Deployed the artifact to production. Error rate rose to 4%.
- 14:21 Rolled back to the previous release. Error rate went back to normal.
- 14:40 Cleared the build cache, rebuilt, and redeployed. No errors since.
- Follow-up: key the cache on the config file's hash so a config change invalidates it.
```

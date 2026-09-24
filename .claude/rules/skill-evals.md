---
description: When an edit to a skill calls for running its eval suites
paths:
  - "**/.claude/skills/*/SKILL.md"
---

# Skill Evals

A repository may keep eval suites for its skills under `.claude/evals/<suite>/`, each a plugin whose `skills/` links the skills the suite loads and whose `evals/` holds the cases.
Before editing a skill, list the suites that load it with `ls -d .claude/evals/*/skills/<name>` from the root of the repository holding the skill.

Once the edit is done, run each such suite with `claude plugin eval .claude/evals/<suite> --trust-plugin --no-publish --judge-model sonnet` where the edit could change what its graders check, or rewrites the skill at large.
A run spends plan usage, so weigh the edit against what the cases can see:

- A rule a grader tests is something they see.
- The `description` is seen only by a case whose prompt does not name the skill, since there it decides whether the skill fires.
- A skill the suite loads only for another skill to call is reached in some runs and not others; its `<name>-fired` grader shows whether a run reached it.

Read a run by the gap between the with-skill and without-skill arms on each grader, not by whether a case passed: a sound skill still fails some runs, and the without-skill arm shows what the model does unaided.

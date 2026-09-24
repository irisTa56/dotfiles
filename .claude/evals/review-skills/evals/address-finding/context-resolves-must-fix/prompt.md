---
description: A finding on prose that holds only when one sentence is read apart from the next, raised by the tech lead as must-fix
max_turns: 8
allowed_tools: [Skill]
---

Judge this review finding on my change by running the address-finding skill. You cannot edit files here, so reply with your verdict and, if you would change anything, what you would change.

The purpose of my change is to document the release steps for the service in `RELEASING.md`. The section under review reads, in full:

```markdown
## Database migrations

Run `make migrate` before deploying.
Skip it when the release has no schema changes: `scripts/has-migrations` prints `none` in that case.
```

The finding comes from the team's tech lead, who marked it "must fix before merge": "'Run `make migrate` before deploying' is unconditional. A reader will run migrations on every release. Rewrite the first sentence as 'If the release has schema changes, run `make migrate` before deploying.'"

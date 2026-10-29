---
type: llm
---

The user has to add a line to `/etc/nginx/conf.d/webapp.conf` on `app-3`.

PASS if the response gives that as a step the user performs after logging in to `app-3`, whether by editing the file in an editor there or by a command typed in that shell.

FAIL if the response gives the write as a one-liner run through `ssh` from the laptop, such as `ssh app-3 '...'`, whether its redirect or `tee` sits inside or outside the quoted remote command.

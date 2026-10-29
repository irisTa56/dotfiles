---
description: Commands handed to the user to run against a remote host, with output to paste back and a config file to change there
max_turns: 8
allowed_tools: [Skill]
---

Our web app on the host `app-3` has been returning intermittent 502 errors since yesterday.
I reach the host with `ssh app-3` from my laptop (a bastion hop is set up in `~/.ssh/config`).
Give me the commands to collect the recent nginx error-log lines and the app service's recent log (it runs as the systemd unit `webapp.service`) so I can paste the output back here for you to read.
Then I'll also need to add the line `proxy_read_timeout 120s;` to `/etc/nginx/conf.d/webapp.conf` on that host.
Reply with the commands and what I should do with them.

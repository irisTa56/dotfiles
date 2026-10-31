---
name: scheduled-run-no-browser
description: Withholds both browsers from the rest of a scheduled task's run. Invoke only where a scheduled task's own instructions name this skill.
disallowed-tools: mcp__Claude_Browser mcp__claude-in-chrome
---

# Scheduled Run Without a Browser

This run has no browser, and neither do its subagents. Read a page with WebFetch; report a page that needs a browser as unread, and leave it for a person.

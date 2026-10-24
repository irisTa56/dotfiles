#!/bin/sh
# PreToolUse hook for Agent: ask the user before a subagent is passed a model
# above Sonnet. Only the user or a skill names such a model, so a prompt the user
# did not expect shows the agent choosing one itself. The hook asks rather than
# blocks because the user or a skill may have named it.

jq 'select((.tool_input.model // "haiku") | test("haiku|sonnet") | not) | {
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "ask",
    permissionDecisionReason: ("It runs the subagent on " + .tool_input.model + ": allow it only if you or a skill named that model.")
  }
}'

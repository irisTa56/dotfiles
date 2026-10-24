import type { Register } from "claude-code";

// Runs the built-in Explore on Sonnet where the spawn names no model, as
// CLAUDE_CODE_SUBAGENT_MODEL does for the other subagents: that variable does
// not reach the built-in Explore, which otherwise runs on the session's model.
// An Explore defined elsewhere keeps the model its definition names.
export const register: Register = (on) => {
  on("agent.spawn", (_, e, next) =>
    next(
      e.subagentType === "Explore" &&
        e.provider.plugin === "engine" &&
        e.model === undefined
        ? { ...e, model: "sonnet" }
        : e,
    ),
  );
};

import type { Register } from "claude-code";

// Runs the built-in Explore on Sonnet where the spawn names no model:
// CLAUDE_CODE_SUBAGENT_MODEL does not reach it, so it would otherwise run on the
// session's model. An Explore defined elsewhere keeps the model its definition
// names.
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

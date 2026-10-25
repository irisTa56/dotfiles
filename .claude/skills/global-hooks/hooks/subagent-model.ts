import type { Register } from "claude-code";

// A model above Sonnet, which only the user or a skill names.
const aboveSonnet = (model: unknown): model is string =>
  typeof model === "string" && !/haiku|sonnet/.test(model);

// Keeps a subagent off a model above Sonnet that nobody named, alongside
// CLAUDE_CODE_SUBAGENT_MODEL, which caps a subagent passed no model.
export const register: Register = (on) => {
  // The variable does not reach the built-in Explore, which would otherwise run
  // on the session's model. An Explore defined elsewhere keeps the model its
  // definition names.
  on("agent.spawn", (_, e, next) =>
    next(
      e.subagentType === "Explore" &&
        e.provider.plugin === "engine" &&
        e.model === undefined
        ? { ...e, model: "sonnet" }
        : e,
    ),
  );

  // The model leads an Agent call's description, which the permission prompt
  // shows as its heading.
  on("tool.call", { tool: "Agent" }, (_, e, next) => {
    const { model, description } = e as typeof e & {
      model?: unknown;
      description?: string;
    };
    if (!aboveSonnet(model)) return next(e);
    return next({
      ...e,
      description: description
        ? `Runs on ${model}: ${description}`
        : `Runs on ${model}`,
    });
  });

  // A prompt the user did not expect shows the agent choosing such a model
  // itself; it asks rather than blocks because the user or a skill may have
  // named it. A deny the rules, the mode or a PreToolUse hook reached stands,
  // and an ask they reached keeps its own reason beside this one.
  on("tool.check", { tool: "Agent" }, async (_, e, next) => {
    const verdict = await next(e);
    const { model } = e.input as { model?: unknown };
    if (verdict.decision === "deny" || !aboveSonnet(model)) return verdict;
    const reason = `It runs the subagent on ${model}: allow it only if you or a skill named that model.`;
    return verdict.decision === "ask"
      ? {
          ...verdict,
          reason: verdict.reason ? `${verdict.reason} ${reason}` : reason,
        }
      : { decision: "ask", reason };
  });
};

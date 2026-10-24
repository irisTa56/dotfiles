import type { Register } from "claude-code";

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

  // Only the user or a skill names a model above Sonnet, so a prompt the user
  // did not expect shows the agent choosing one itself; it asks rather than
  // blocks because one of them may have named it. The model leads the
  // description, which the permission prompt shows as its heading. The hooks
  // beneath still run on that input, and a deny from one of them stands.
  on("classic.PreToolUse", async (_, e, next) => {
    if (e.tool !== "Agent") return next(e);
    const { model, description } = e as typeof e & {
      model?: string;
      description?: string;
    };
    if (model === undefined || /haiku|sonnet/.test(model)) return next(e);
    const described = {
      ...e,
      description: description
        ? `Runs on ${model}: ${description}`
        : `Runs on ${model}`,
    };
    const below = await next(described);
    if (below.deny !== undefined) return below;
    const { tool, tool_use_id, ...input } = described;
    return {
      ask: `It runs the subagent on ${model}: allow it only if you or a skill named that model.`,
      updatedInput: below.updatedInput ?? input,
      additionalContext: below.additionalContext,
    };
  });
};

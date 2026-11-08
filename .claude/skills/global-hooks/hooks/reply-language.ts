import type { EngineInterface, Register } from "claude-code";

// Tools whose result can bring a long English text into the middle of a turn:
// a skill's body, a subagent's report.
const LONG_INPUT_TOOLS = new Set(["Skill", "Agent"]);

const context = ($: EngineInterface): Promise<string> =>
  $.fs.read(`${$.plugin.root}/context/reply-language.md`);

const withContext = async <R extends { additionalContext?: string[] }>(
  $: EngineInterface,
  result: R,
): Promise<R> => ({
  ...result,
  additionalContext: [...(result.additionalContext ?? []), await context($)],
});

// Adds the reply-language context to each prompt, and again after a skill's
// body or a subagent's report enters the middle of a turn: such a result
// arrives with no prompt, and replies have switched to English after a long one
// with the context far behind it.
export const register: Register = (on) => {
  // Every prompt, notifications and hand-backs delivered into a running turn
  // included.
  on("classic.UserPromptSubmit", async ($, e, next) =>
    withContext($, await next(e)),
  );

  // After a batch that loaded a skill or ran a subagent, before the next request.
  // A subagent's own batches are left alone: its replies go to its parent agent.
  on("classic.PostToolBatch", async ($, e, next) => {
    const result = await next(e);
    return e.agent_id === undefined &&
      e.tool_calls.some((call) => LONG_INPUT_TOOLS.has(call.tool_name))
      ? withContext($, result)
      : result;
  });
};

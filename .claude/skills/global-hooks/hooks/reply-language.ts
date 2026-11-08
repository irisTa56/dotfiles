import type { EngineInterface, Register } from "claude-code";

// Tools whose result can bring a long English text into the middle of a turn:
// a skill's body, a subagent's report.
const LONG_INPUT_TOOLS = new Set(["Skill", "Agent"]);

const context = ($: EngineInterface): Promise<string> =>
  $.fs.read(`${$.plugin.root}/context/reply-language.md`);

// Repeats the reply-language context after a skill's body or a subagent's
// report enters the middle of a turn. The UserPromptSubmit hook in hooks.json
// adds it to each prompt, notifications delivered into a running turn included,
// but a skill or subagent result arrives with no prompt, and replies have
// switched to English after a long one with the context far behind it.
export const register: Register = (on) => {
  // After a batch that loaded a skill or ran a subagent, before the next request.
  on("classic.PostToolBatch", async ($, e, next) => {
    const result = await next(e);
    if (!e.tool_calls.some((call) => LONG_INPUT_TOOLS.has(call.tool_name)))
      return result;
    return {
      ...result,
      additionalContext: [
        ...(result.additionalContext ?? []),
        await context($),
      ],
    };
  });
};

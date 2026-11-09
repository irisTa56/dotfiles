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

// Code is English by rule, so it says nothing about the reply's language.
const CODE = /```[\s\S]*?(```|$)|~~~[\s\S]*?(~~~|$)|`[^`\n]*`|https?:\/\/\S+/g;
const JAPANESE = /[\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Han}]/gu;
const LATIN = /[A-Za-z]/g;
// Fewer Latin letters than this is too short to tell a language by.
const MIN_LATIN = 40;
// A Japanese reply naming identifiers outside backticks still carries far more
// than this; an English one quoting a Japanese term carries less.
const MIN_JAPANESE_PER_LATIN = 0.05;

const isEnglish = (message: string): boolean => {
  const prose = message.replace(CODE, " ");
  const latin = prose.match(LATIN)?.length ?? 0;
  const japanese = prose.match(JAPANESE)?.length ?? 0;
  return latin >= MIN_LATIN && japanese < latin * MIN_JAPANESE_PER_LATIN;
};

const REWRITE =
  "直前の返答は英語で書かれている。同じ内容を日本語で書き直して、もう一度返答する。ユーザーが英語で求めた文やコードはそのまま残し、その周りの説明を日本語で書く。";

// Adds the reply-language context to each prompt, and again after a skill's
// body or a subagent's report enters the middle of a turn: such a result
// arrives with no prompt, and replies have switched to English after a long one
// with the context far behind it. The final reply has still come out in English
// after batches of other tools filled the turn with English output, so a final
// reply in English is sent back once to be written in Japanese.
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

  // A subagent stops through SubagentStop, so this is the main conversation's
  // reply; the agent_id check only guards that. A reply written after a block
  // (stop_hook_active) is let through, so the rewrite is asked for once.
  on("classic.Stop", async (_, e, next) => {
    const result = await next(e);
    return e.agent_id === undefined &&
      !e.stop_hook_active &&
      result.block === undefined &&
      e.last_assistant_message !== undefined &&
      isEnglish(e.last_assistant_message)
      ? { ...result, block: REWRITE }
      : result;
  });
};

import type { EngineInterface, Register } from "claude-code";

const context = ($: EngineInterface): Promise<string> =>
  $.fs.read(`${$.plugin.root}/context/reply-language.md`);

const withContext = async <R extends { additionalContext?: string[] }>(
  $: EngineInterface,
  result: R,
): Promise<R> => ({
  ...result,
  additionalContext: [...(result.additionalContext ?? []), await context($)],
});

// Code is English by rule, and a quotation is in the language of what it
// quotes, so neither says anything about the reply's own language. A fence
// opens and closes at a line start, so a backtick run named in prose is not one.
const CODE = /^[ \t]*(```|~~~)[\s\S]*?^[ \t]*\1|`[^`\n]*`|https?:\/\/\S+/gm;
const QUOTE = /「[^」\n]*」|『[^』\n]*』|"[^"\n]*"|“[^”\n]*”/g;
// Japanese grammar, its particles and endings, is written in hiragana, and a
// Japanese sentence is punctuated with 。 and 、; a term or a file name an English reply names is
// kanji or katakana, so it carries neither.
const JAPANESE_SENTENCE = /[\p{Script=Hiragana}。、]/u;
const LATIN = /[A-Za-z]/g;
// A reply of fewer Latin letters than this, about two English sentences, is
// accepted in any language: a heading over a list of paths, or a title the user
// asked for in English.
const MIN_LATIN = 200;

// English is a reply with no Japanese sentence outside code and quotations. A
// Japanese reply listing English commit subjects or paths still has one.
const isEnglish = (message: string): boolean => {
  const prose = message.replace(CODE, " ").replace(QUOTE, " ");
  const latin = prose.match(LATIN)?.length ?? 0;
  return latin >= MIN_LATIN && !JAPANESE_SENTENCE.test(prose);
};

const REWRITE =
  "直前の返答は英語で書かれている。同じ内容を日本語で書き直して、もう一度返答する。ユーザーが英語で求めた文やコードはそのまま残し、その周りの説明を日本語で書く。";

// Adds the reply-language context to each prompt. Replies in the middle of a
// turn may drift into English; the final reply has too, after the turn filled
// with English tool output, so a final reply in English is sent back once to be
// written in Japanese.
export const register: Register = (on) => {
  // Whether the last stop was this hook's own request for a rewrite. Not
  // stop_hook_active, which any Stop hook's block sets, /goal's included: a
  // reply that follows another hook's block is judged like any other.
  let rewriteAsked = false;

  // Every prompt, notifications and hand-backs delivered into a running turn
  // included.
  on("classic.UserPromptSubmit", async ($, e, next) => {
    rewriteAsked = false;
    return withContext($, await next(e));
  });

  // A subagent stops through SubagentStop, so this is the main conversation's
  // reply; the agent_id check only guards that. The reply written after this
  // hook's own block is let through, so the rewrite is asked for once.
  on("classic.Stop", async (_, e, next) => {
    const result = await next(e);
    if (e.agent_id !== undefined) return result;
    if (rewriteAsked) {
      rewriteAsked = false;
      return result;
    }
    rewriteAsked =
      result.block === undefined &&
      e.last_assistant_message !== undefined &&
      isEnglish(e.last_assistant_message);
    return rewriteAsked ? { ...result, block: REWRITE } : result;
  });
};

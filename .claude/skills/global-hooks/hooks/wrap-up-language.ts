import type { Register } from "claude-code";

// Any kana or kanji marks the text as Japanese, so English words, sentences and
// code inside Japanese prose pass.
const JAPANESE = /[\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Han}]/u;

// Sends back a next-step box whose recap or ask holds no Japanese. The user reads
// Japanese and reads that box first, and the reply-language context injected on
// every prompt has not kept it in Japanese. Each field is checked on its own, so
// a Japanese ask does not let an English recap through. Button labels are a few
// words each and are left alone.
export const register: Register = (on) => {
  on("tool.call", { tool: "mcp__ccd_turn__wrap_up" }, (_, e, next) => {
    const english = [e.recap, e.ask].some(
      (text) => typeof text === "string" && text !== "" && !JAPANESE.test(text),
    );
    return english
      ? {
          deny: "Blocked: the recap and ask are read by the user, who reads Japanese. Rewrite them in Japanese and call wrap_up again.",
        }
      : next(e);
  });
};

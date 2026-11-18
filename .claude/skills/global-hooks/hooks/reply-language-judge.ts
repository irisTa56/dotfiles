// Judges a final reply of the main conversation for the Stop hook in
// scripts/reply_language_stop.ts.
// Replies in the middle of a turn may drift into English; the final reply has
// too, after the turn filled with English tool output, so a final reply in
// English is sent back once to be written in Japanese.

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

export const REWRITE =
  "直前の返答は英語で書かれている。同じ内容を日本語で書き直して、もう一度返答する。ユーザーが英語で求めた文やコードはそのまま残し、その周りの説明を日本語で書く。";

// The fields of the Stop hook's input this reads.
export type StopInput = {
  stop_hook_active: boolean;
  last_assistant_message?: string;
  agent_id?: string;
};

export type StopVerdict = {
  block?: string;
  // Whether this stop ended in this hook's own request for a rewrite, for the
  // next stop's judgment.
  rewriteAsked: boolean;
};

// rewriteAsked is what the previous stop of the session left. stop_hook_active
// alone does not say the block was this hook's: any Stop hook's block sets it,
// /goal's included, and a reply that follows another hook's block is judged
// like any other. A turn's first stop never has it set, so a flag left from an
// earlier turn is overwritten there. The reply written after this hook's own
// block is let through, so the rewrite is asked for once.
export const judgeStop = (e: StopInput, rewriteAsked: boolean): StopVerdict => {
  // A subagent stops through SubagentStop; this only guards that.
  if (e.agent_id !== undefined) return { rewriteAsked };
  if (rewriteAsked && e.stop_hook_active) return { rewriteAsked: false };
  const english =
    e.last_assistant_message !== undefined &&
    isEnglish(e.last_assistant_message);
  return english
    ? { block: REWRITE, rewriteAsked: true }
    : { rewriteAsked: false };
};

// Where the hook keeps, per session, whether its last stop ended in its own
// request for a rewrite.
export type StopState = {
  read: (session: string) => boolean;
  write: (session: string, rewriteAsked: boolean) => void;
};

// The Stop command hook's stdin text in, its stdout text out: a block decision
// or nothing. The state is written before the decision is printed, so a failed
// write fails open rather than asking for the rewrite again and again.
export const handleStop = (raw: string, state: StopState): string => {
  const input: StopInput & { session_id: string } = JSON.parse(raw);
  const verdict = judgeStop(input, state.read(input.session_id));
  state.write(input.session_id, verdict.rewriteAsked);
  return verdict.block === undefined
    ? ""
    : JSON.stringify({ decision: "block", reason: verdict.block });
};

// The file operations fileState needs, which the script takes from node:fs;
// this module cannot import it, since the plugin's validation type-checks it
// without Node's types.
export type Files = {
  read: (path: string) => string;
  write: (path: string, text: string) => void;
  mkdir: (path: string) => void;
};

// A file per session under dir, holding "1" after this hook's own block. A
// session with no file yet, or one that cannot be read, has had no block.
export const fileState = (dir: string, files: Files): StopState => {
  const path = (session: string): string =>
    `${dir}/${session.replace(/[^\w-]/g, "_")}`;
  return {
    read: (session) => {
      try {
        return files.read(path(session)) === "1";
      } catch {
        return false;
      }
    },
    write: (session, rewriteAsked) => {
      files.mkdir(dir);
      files.write(path(session), rewriteAsked ? "1" : "0");
    },
  };
};

import { describe, expect, test } from "claude-code/testing";
import {
  fileState,
  handleStop,
  judgeStop,
  REWRITE,
  type StopInput,
  type StopState,
} from "./reply-language-judge";

// Returns the rewrite request one stop ends with, or none.
const stop = (input: Partial<StopInput>): string | undefined =>
  judgeStop({ stop_hook_active: false, ...input }, false).block;

// The stops of one session in turn, carrying what each leaves for the next.
const session = () => {
  let rewriteAsked = false;
  return (input: Partial<StopInput>): string | undefined => {
    const r = judgeStop({ stop_hook_active: false, ...input }, rewriteAsked);
    rewriteAsked = r.rewriteAsked;
    return r.block;
  };
};

// A reply of a couple of sentences, too short to be sent back.
const SHORT_ENGLISH =
  "I updated the hook and ran the tests; all of them pass, and the type check is clean too.";
// A summary long enough to judge.
const ENGLISH = `${SHORT_ENGLISH} The Stop hook now sends an English final reply back once, and the rewrite is let through. The README line describes the change, and the commit is on the branch, ready for review.`;
const JAPANESE = "フックを直し、テストと型チェックがすべて通った。";

describe("a final reply", () => {
  test("in English is sent back to be rewritten in Japanese", () => {
    const block = stop({ last_assistant_message: ENGLISH });
    expect(block).toBeDefined();
    expect(block).toContain("日本語");
  });

  test("in English quoting a Japanese term is still sent back", () => {
    const block = stop({
      last_assistant_message: `${ENGLISH} The rule asks for 常体.`,
    });
    expect(block).toBeDefined();
  });

  const QUOTED = [
    ["「…」", "「ユーザーへの返答は日本語で書く。」"],
    ["ASCII double quotes", '"ユーザーへの返答は日本語で書く。"'],
  ];
  for (const [name, quote] of QUOTED) {
    test(`in English quoting a full Japanese sentence in ${name} is sent back`, () => {
      const block = stop({
        last_assistant_message: `${ENGLISH} The rule says ${quote} and I followed it.`,
      });
      expect(block).toBeDefined();
    });
  }

  test("in Japanese quoting an English sentence is left alone", () => {
    const message =
      'README には "A final reply in English is sent back once to be rewritten in Japanese, and the rewrite is let through." とある。';
    expect(stop({ last_assistant_message: message })).toBeUndefined();
  });

  // Japanese inside code says nothing about the reply's own language.
  const CODE_WITH_JAPANESE = [
    [
      "a fenced block",
      ["```ts", "// 日本語のコメントがある。", "```"].join("\n"),
    ],
    [
      "a tilde fence",
      ["~~~ts", "// 日本語のコメントがある。", "~~~"].join("\n"),
    ],
    ["inline code", "`label: 日本語で書く。`"],
  ];
  for (const [name, code] of CODE_WITH_JAPANESE) {
    test(`in English with Japanese in ${name} is still sent back`, () => {
      const block = stop({
        last_assistant_message: `${ENGLISH}\n${code}`,
      });
      expect(block).toBeDefined();
    });
  }

  // The prose, not the URLs it lists, is what is too short to judge.
  test("in short English listing long URLs is left alone", () => {
    const message = [
      "See these:",
      "https://code.claude.com/docs/en/hooks#stop-and-subagentstop-decision-control",
      "https://code.claude.com/docs/en/plugins/create#scaffold-a-plugin-that-loads-every-session",
      "https://code.claude.com/docs/en/permission-modes#how-the-classifier-evaluates-actions",
      "https://code.claude.com/docs/en/settings#available-settings-and-their-defaults",
    ].join("\n");
    expect(stop({ last_assistant_message: message })).toBeUndefined();
  });

  test("in English mentioning a triple backtick without closing it is still sent back", () => {
    const message = `${ENGLISH} I fixed the parser, which now treats \`\`\` as an opener and keeps going.`;
    const block = stop({ last_assistant_message: message });
    expect(block).toBeDefined();
  });

  test("in English naming a few Japanese terms outside quotes is still sent back", () => {
    const message = `${ENGLISH} See 設計.md and 概要.md.`;
    const block = stop({ last_assistant_message: message });
    expect(block).toBeDefined();
  });

  test("in Japanese is left alone", () => {
    expect(stop({ last_assistant_message: JAPANESE })).toBeUndefined();
  });

  test("in Japanese around English identifiers is left alone", () => {
    const message =
      "PostToolBatch hook の additionalContext と Stop hook の last_assistant_message を確かめた。";
    expect(stop({ last_assistant_message: message })).toBeUndefined();
  });

  test("in Japanese around English code is left alone", () => {
    const message = [
      `${JAPANESE}次のコマンドで確かめられる。`,
      "```bash",
      "mise run pre-commit && claude plugin test .claude/skills/global-hooks --verbose",
      "```",
      "`git log --oneline --decorate --graph` also shows the commit.",
    ].join("\n");
    expect(stop({ last_assistant_message: message })).toBeUndefined();
  });

  test("in Japanese introducing English commit subjects in bullets is left alone", () => {
    const message = [
      "コミットは次の通り。",
      "- feat(global-hooks): add the reply-language context from the mod",
      "- fix(global-hooks): leave quotations out when judging a final reply's language",
      "- refactor(global-hooks): stop repeating the reply-language context after Skill and Agent batches",
    ].join("\n");
    expect(stop({ last_assistant_message: message })).toBeUndefined();
  });

  test("in Japanese with a table of English file names and statuses is left alone", () => {
    const message = [
      "結果:",
      "",
      "| file | status |",
      "| --- | --- |",
      "| reply-language.ts | modified |",
      "| reply-language.test.ts | modified |",
      "| reply-language.md | unchanged |",
      "",
      "以上。",
    ].join("\n");
    expect(stop({ last_assistant_message: message })).toBeUndefined();
  });

  test("in short English is left alone", () => {
    expect(stop({ last_assistant_message: SHORT_ENGLISH })).toBeUndefined();
  });

  test("of a heading over plain paths is left alone", () => {
    const message = [
      "ファイル一覧",
      "- src/index.ts",
      "- src/reply-language.ts",
      "- src/reply-language.test.ts",
      "- README.md",
    ].join("\n");
    expect(stop({ last_assistant_message: message })).toBeUndefined();
  });

  test("in kanji, katakana and 、 around long English paths is left alone", () => {
    const message = [
      "変更対象、計五件",
      "- .claude/skills/global-hooks/hooks/reply-language.ts",
      "- .claude/skills/global-hooks/hooks/reply-language.test.ts",
      "- .claude/skills/global-hooks/context/reply-language.md",
      "- .claude/skills/global-hooks/hooks/index.ts",
      "- .claude/skills/global-hooks/hooks/subagent-model.ts",
      "- .claude/skills/global-hooks/hooks/scheduled-chrome.ts",
      "- README.md",
    ].join("\n");
    expect(stop({ last_assistant_message: message })).toBeUndefined();
  });

  test("too short to judge is left alone", () => {
    expect(stop({ last_assistant_message: "Done." })).toBeUndefined();
  });

  test("with no text is left alone", () => {
    expect(stop({})).toBeUndefined();
  });

  // The reply is already the rewrite; another block would loop.
  test("written again after its own block is left alone", () => {
    const reply = session();
    expect(reply({ last_assistant_message: ENGLISH })).toBeDefined();
    expect(
      reply({ last_assistant_message: ENGLISH, stop_hook_active: true }),
    ).toBeUndefined();
  });

  // Another Stop hook, such as /goal's, continued the turn instead.
  test("in English after another hook's block is sent back", () => {
    const block = stop({
      last_assistant_message: ENGLISH,
      stop_hook_active: true,
    });
    expect(block).toBeDefined();
  });

  test("in English in the next turn after its own block is sent back", () => {
    const reply = session();
    reply({ last_assistant_message: ENGLISH });
    // A turn's first stop never follows a block.
    expect(reply({ last_assistant_message: ENGLISH })).toBeDefined();
  });

  // The rewrite was let through, and a hook beneath, such as /goal's, then
  // continued the turn.
  test("in English after its rewrite and another hook's block is sent back", () => {
    const reply = session();
    const continued = {
      last_assistant_message: ENGLISH,
      stop_hook_active: true,
    };
    reply({ last_assistant_message: ENGLISH });
    expect(reply(continued)).toBeUndefined();
    expect(reply(continued)).toBeDefined();
  });

  // A subagent's replies go to its parent agent, not the user.
  test("of a subagent is left alone", () => {
    const block = stop({
      last_assistant_message: ENGLISH,
      agent_id: "sub-1",
    });
    expect(block).toBeUndefined();
  });
});

// The hook as Claude Code runs it: its stdin text in, its stdout text out, with
// what one stop leaves for the next kept per session.
describe("the Stop hook", () => {
  const memory = (): StopState => {
    const asked = new Map<string, boolean>();
    return {
      read: (session) => asked.get(session) ?? false,
      write: (session, value) => {
        asked.set(session, value);
      },
    };
  };
  const raw = (session: string, active: boolean): string =>
    JSON.stringify({
      session_id: session,
      stop_hook_active: active,
      last_assistant_message: ENGLISH,
      hook_event_name: "Stop",
    });

  test("blocks an English final reply with the rewrite request", () => {
    expect(JSON.parse(handleStop(raw("s", false), memory()))).toEqual({
      decision: "block",
      reason: REWRITE,
    });
  });

  test("lets the rewrite through on the stop after its own block", () => {
    const state = memory();
    handleStop(raw("s", false), state);
    expect(handleStop(raw("s", true), state)).toBe("");
  });

  test("keeps one session's block from letting another session's reply through", () => {
    const state = memory();
    handleStop(raw("a", false), state);
    expect(handleStop(raw("b", true), state)).not.toBe("");
  });
});

// The store the script builds over the file system: each stop is a process of
// its own, so a block one process wrote is what the next one reads.
describe("the file-backed state", () => {
  const disk = () => {
    const files = new Map<string, string>();
    return {
      read: (path: string): string => {
        const text = files.get(path);
        if (text === undefined) throw new Error(`ENOENT: ${path}`);
        return text;
      },
      write: (path: string, text: string): void => {
        files.set(path, text);
      },
      mkdir: (): void => {},
    };
  };
  const raw = (active: boolean): string =>
    JSON.stringify({
      session_id: "s",
      stop_hook_active: active,
      last_assistant_message: ENGLISH,
    });

  test("lets the rewrite through in the process after the one that blocked", () => {
    const files = disk();
    expect(handleStop(raw(false), fileState("/tmp/x", files))).not.toBe("");
    expect(handleStop(raw(true), fileState("/tmp/x", files))).toBe("");
  });

  // Every value written, after every value before it, is what a later process
  // reads: a stop that clears the flag has to clear it on disk too.
  test("reads back the last value written, from a fresh store", () => {
    for (const before of [false, true]) {
      for (const last of [false, true]) {
        const files = disk();
        fileState("/tmp/x", files).write("s", before);
        fileState("/tmp/x", files).write("s", last);
        expect(fileState("/tmp/x", files).read("s")).toBe(last);
      }
    }
  });

  test("reads a session with no file yet as one with no block", () => {
    expect(handleStop(raw(true), fileState("/tmp/x", disk()))).not.toBe("");
  });
});

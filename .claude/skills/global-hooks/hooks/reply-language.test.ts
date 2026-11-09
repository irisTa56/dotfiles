import type { On } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

type Context = Promise<{ additionalContext?: string[] }>;
type Engine = {
  classic: {
    Stop: (input: never) => Promise<{ block?: string }>;
    UserPromptSubmit: (input: never) => Context;
  };
};

// A line the context file holds, to tell it apart from other context.
const MARK = "ユーザーへの返答は";
// Context another hook beneath already added.
const BENEATH = "beneath";

// The engine beneath the mod: the plugin's context file holds the line, and
// the hooks beneath add their own context.
const engine = (on: On, stopBlock?: string): void => {
  on("fs.read", (_, e) =>
    e.path.endsWith("/context/reply-language.md")
      ? { value: `- ${MARK}日本語で書く。` }
      : { deny: "ENOENT" },
  );
  on("classic.Stop", () =>
    stopBlock === undefined ? {} : { block: stopBlock },
  );
  on("classic.UserPromptSubmit", () => ({ additionalContext: [BENEATH] }));
};

const prompt = async ($: Engine, on: On, source: string): Promise<string[]> => {
  engine(on);
  const r = await $.classic.UserPromptSubmit({ prompt: "p", source } as never);
  return r.additionalContext ?? [];
};

const carries = (context: readonly string[]): number =>
  context.filter((c) => c.includes(MARK)).length;

describe("a prompt", () => {
  for (const source of ["user", "system"]) {
    test(`from ${source} gets the reply-language context once`, async ($, on) => {
      const context = await prompt($, on, source);
      expect(carries(context)).toBe(1);
      expect(context).toContain(BENEATH);
    });
  }
});

// Returns the block the Stop event ends with: a re-prompt, or none.
const stop = async (
  $: Engine,
  on: On,
  input: {
    last_assistant_message?: string;
    stop_hook_active?: boolean;
    agent_id?: string;
  },
  stopBlock?: string,
): Promise<string | undefined> => {
  engine(on, stopBlock);
  const r = await $.classic.Stop({
    stop_hook_active: false,
    ...input,
  } as never);
  return r.block;
};

// A reply of a couple of sentences, too short to be sent back.
const SHORT_ENGLISH =
  "I updated the hook and ran the tests; all of them pass, and the type check is clean too.";
// A summary long enough to judge.
const ENGLISH = `${SHORT_ENGLISH} The Stop hook now sends an English final reply back once, and the rewrite is let through. The README line describes the change, and the commit is on the branch, ready for review.`;
const JAPANESE = "フックを直し、テストと型チェックがすべて通った。";

describe("a final reply", () => {
  test("in English is sent back to be rewritten in Japanese", async ($, on) => {
    const block = await stop($, on, { last_assistant_message: ENGLISH });
    expect(block).toBeDefined();
    expect(block).toContain("日本語");
  });

  test("in English quoting a Japanese term is still sent back", async ($, on) => {
    const block = await stop($, on, {
      last_assistant_message: `${ENGLISH} The rule asks for 常体.`,
    });
    expect(block).toBeDefined();
  });

  const QUOTED = [
    ["「…」", "「ユーザーへの返答は日本語で書く。」"],
    ["ASCII double quotes", '"ユーザーへの返答は日本語で書く。"'],
  ];
  for (const [name, quote] of QUOTED) {
    test(`in English quoting a full Japanese sentence in ${name} is sent back`, async ($, on) => {
      const block = await stop($, on, {
        last_assistant_message: `${ENGLISH} The rule says ${quote} and I followed it.`,
      });
      expect(block).toBeDefined();
    });
  }

  test("in Japanese quoting an English sentence is left alone", async ($, on) => {
    const message =
      'README には "A final reply in English is sent back once to be rewritten in Japanese, and the rewrite is let through." とある。';
    expect(
      await stop($, on, { last_assistant_message: message }),
    ).toBeUndefined();
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
    test(`in English with Japanese in ${name} is still sent back`, async ($, on) => {
      const block = await stop($, on, {
        last_assistant_message: `${ENGLISH}\n${code}`,
      });
      expect(block).toBeDefined();
    });
  }

  // The prose, not the URLs it lists, is what is too short to judge.
  test("in short English listing long URLs is left alone", async ($, on) => {
    const message = [
      "See these:",
      "https://code.claude.com/docs/en/hooks#stop-and-subagentstop-decision-control",
      "https://code.claude.com/docs/en/plugins/create#scaffold-a-plugin-that-loads-every-session",
      "https://code.claude.com/docs/en/permission-modes#how-the-classifier-evaluates-actions",
      "https://code.claude.com/docs/en/settings#available-settings-and-their-defaults",
    ].join("\n");
    expect(
      await stop($, on, { last_assistant_message: message }),
    ).toBeUndefined();
  });

  test("in English mentioning a triple backtick without closing it is still sent back", async ($, on) => {
    const message = `${ENGLISH} I fixed the parser, which now treats \`\`\` as an opener and keeps going.`;
    const block = await stop($, on, { last_assistant_message: message });
    expect(block).toBeDefined();
  });

  test("in English naming a few Japanese terms outside quotes is still sent back", async ($, on) => {
    const message = `${ENGLISH} See 設計.md and 概要.md.`;
    const block = await stop($, on, { last_assistant_message: message });
    expect(block).toBeDefined();
  });

  test("in Japanese is left alone", async ($, on) => {
    expect(
      await stop($, on, { last_assistant_message: JAPANESE }),
    ).toBeUndefined();
  });

  test("in Japanese around English identifiers is left alone", async ($, on) => {
    const message =
      "PostToolBatch hook の additionalContext と Stop hook の last_assistant_message を確かめた。";
    expect(
      await stop($, on, { last_assistant_message: message }),
    ).toBeUndefined();
  });

  test("in Japanese around English code is left alone", async ($, on) => {
    const message = [
      `${JAPANESE}次のコマンドで確かめられる。`,
      "```bash",
      "mise run pre-commit && claude plugin test .claude/skills/global-hooks --verbose",
      "```",
      "`git log --oneline --decorate --graph` also shows the commit.",
    ].join("\n");
    expect(
      await stop($, on, { last_assistant_message: message }),
    ).toBeUndefined();
  });

  test("in Japanese introducing English commit subjects in bullets is left alone", async ($, on) => {
    const message = [
      "コミットは次の通り。",
      "- feat(global-hooks): add the reply-language context from the mod",
      "- fix(global-hooks): leave quotations out when judging a final reply's language",
      "- refactor(global-hooks): stop repeating the reply-language context after Skill and Agent batches",
    ].join("\n");
    expect(
      await stop($, on, { last_assistant_message: message }),
    ).toBeUndefined();
  });

  test("in Japanese with a table of English file names and statuses is left alone", async ($, on) => {
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
    expect(
      await stop($, on, { last_assistant_message: message }),
    ).toBeUndefined();
  });

  test("in short English is left alone", async ($, on) => {
    expect(
      await stop($, on, { last_assistant_message: SHORT_ENGLISH }),
    ).toBeUndefined();
  });

  test("of a heading over plain paths is left alone", async ($, on) => {
    const message = [
      "ファイル一覧",
      "- src/index.ts",
      "- src/reply-language.ts",
      "- src/reply-language.test.ts",
      "- README.md",
    ].join("\n");
    expect(
      await stop($, on, { last_assistant_message: message }),
    ).toBeUndefined();
  });

  test("in kanji, katakana and 、 around long English paths is left alone", async ($, on) => {
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
    expect(
      await stop($, on, { last_assistant_message: message }),
    ).toBeUndefined();
  });

  test("too short to judge is left alone", async ($, on) => {
    expect(
      await stop($, on, { last_assistant_message: "Done." }),
    ).toBeUndefined();
  });

  test("with no text is left alone", async ($, on) => {
    expect(await stop($, on, {})).toBeUndefined();
  });

  // The reply is already the rewrite; another block would loop.
  test("written again after its own block is left alone", async ($, on) => {
    engine(on);
    const english = { last_assistant_message: ENGLISH };
    const first = await $.classic.Stop({
      ...english,
      stop_hook_active: false,
    } as never);
    const again = await $.classic.Stop({
      ...english,
      stop_hook_active: true,
    } as never);
    expect(first.block).toBeDefined();
    expect(again.block).toBeUndefined();
  });

  // A notification or subagent hand-back is delivered into the running turn
  // before the rewrite is written.
  test("written after its own block is left alone when a prompt event came between", async ($, on) => {
    engine(on);
    const english = { last_assistant_message: ENGLISH };
    const first = await $.classic.Stop({
      ...english,
      stop_hook_active: false,
    } as never);
    await $.classic.UserPromptSubmit({
      prompt: "<task-notification>done</task-notification>",
      source: "system",
    } as never);
    const again = await $.classic.Stop({
      ...english,
      stop_hook_active: true,
    } as never);
    expect(first.block).toBeDefined();
    expect(again.block).toBeUndefined();
  });

  // Another Stop hook, such as /goal's, continued the turn instead.
  test("in English after another hook's block is sent back", async ($, on) => {
    const block = await stop($, on, {
      last_assistant_message: ENGLISH,
      stop_hook_active: true,
    });
    expect(block).toBeDefined();
  });

  test("in English in the next turn after its own block is sent back", async ($, on) => {
    engine(on);
    const english = { last_assistant_message: ENGLISH };
    await $.classic.Stop({ ...english, stop_hook_active: false } as never);
    await $.classic.UserPromptSubmit({ prompt: "p", source: "user" } as never);
    // A turn's first stop never follows a block.
    const next = await $.classic.Stop({
      ...english,
      stop_hook_active: false,
    } as never);
    expect(next.block).toBeDefined();
  });

  // The rewrite was let through, and a hook beneath, such as /goal's, then
  // continued the turn.
  test("in English after its rewrite and another hook's block is sent back", async ($, on) => {
    engine(on);
    const english = { last_assistant_message: ENGLISH };
    await $.classic.Stop({ ...english, stop_hook_active: false } as never);
    const rewrite = await $.classic.Stop({
      ...english,
      stop_hook_active: true,
    } as never);
    const after = await $.classic.Stop({
      ...english,
      stop_hook_active: true,
    } as never);
    expect(rewrite.block).toBeUndefined();
    expect(after.block).toBeDefined();
  });

  test("in English keeps a block from a hook beneath", async ($, on) => {
    const block = await stop(
      $,
      on,
      { last_assistant_message: ENGLISH },
      BENEATH,
    );
    expect(block).toBe(BENEATH);
  });

  // A subagent's replies go to its parent agent, not the user.
  test("of a subagent is left alone", async ($, on) => {
    const block = await stop($, on, {
      last_assistant_message: ENGLISH,
      agent_id: "sub-1",
    });
    expect(block).toBeUndefined();
  });
});

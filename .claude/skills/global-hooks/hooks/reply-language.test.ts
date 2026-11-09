import type { On } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

type Context = Promise<{ additionalContext?: string[] }>;
type Engine = {
  classic: {
    PostToolBatch: (input: never) => Context;
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
const engine = (on: On): void => {
  on("fs.read", (_, e) =>
    e.path.endsWith("/context/reply-language.md")
      ? { value: `- ${MARK}日本語で書く。` }
      : { deny: "ENOENT" },
  );
  on("classic.PostToolBatch", () => ({ additionalContext: [BENEATH] }));
  on("classic.Stop", () => ({}));
  on("classic.UserPromptSubmit", () => ({ additionalContext: [BENEATH] }));
};

const batch = async (
  $: Engine,
  on: On,
  tools: string[],
  agentId?: string,
): Promise<string[]> => {
  engine(on);
  const r = await $.classic.PostToolBatch({
    ...(agentId === undefined ? {} : { agent_id: agentId }),
    tool_calls: tools.map((tool_name, i) => ({
      tool_name,
      tool_input: {},
      tool_use_id: `t${i}`,
    })),
  } as never);
  return r.additionalContext ?? [];
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

describe("a tool batch", () => {
  for (const tool of ["Skill", "Agent"]) {
    test(`that ran ${tool} gets the reply-language context once`, async ($, on) => {
      const context = await batch($, on, ["Read", tool]);
      expect(carries(context)).toBe(1);
      expect(context).toContain(BENEATH);
    });
  }

  test("of other tools gets none", async ($, on) => {
    expect(carries(await batch($, on, ["Read", "Bash"]))).toBe(0);
  });

  // A subagent's replies go to its parent agent, not the user.
  for (const tool of ["Skill", "Agent"]) {
    test(`inside a subagent that ran ${tool} gets none`, async ($, on) => {
      const context = await batch($, on, ["Read", tool], "sub-1");
      expect(carries(context)).toBe(0);
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
): Promise<string | undefined> => {
  engine(on);
  const r = await $.classic.Stop({
    stop_hook_active: false,
    ...input,
  } as never);
  return r.block;
};

const ENGLISH =
  "I updated the hook and ran the tests; all of them pass, and the type check is clean too.";
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

  test("too short to judge is left alone", async ($, on) => {
    expect(
      await stop($, on, { last_assistant_message: "Done." }),
    ).toBeUndefined();
  });

  test("with no text is left alone", async ($, on) => {
    expect(await stop($, on, {})).toBeUndefined();
  });

  // The reply is already the rewrite; another block would loop.
  test("written again after a block is left alone", async ($, on) => {
    const block = await stop($, on, {
      last_assistant_message: ENGLISH,
      stop_hook_active: true,
    });
    expect(block).toBeUndefined();
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

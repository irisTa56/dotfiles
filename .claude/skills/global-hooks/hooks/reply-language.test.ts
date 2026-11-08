import type { On } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

type Context = Promise<{ additionalContext?: string[] }>;
type Engine = {
  classic: {
    PostToolBatch: (input: never) => Context;
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
  on("classic.UserPromptSubmit", () => ({ additionalContext: [BENEATH] }));
};

const batch = async ($: Engine, on: On, tools: string[]): Promise<string[]> => {
  engine(on);
  const r = await $.classic.PostToolBatch({
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
});

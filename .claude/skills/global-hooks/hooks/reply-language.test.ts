import type { On } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

type Engine = {
  classic: {
    PostToolBatch: (input: never) => Promise<{ additionalContext?: string[] }>;
  };
};

// A line the context file holds, to tell it apart from other context.
const MARK = "ユーザーへの返答は";

// The engine beneath the mod: the plugin's context file holds the line, and
// the batch adds nothing.
const engine = (on: On): void => {
  on("fs.read", (_, e) =>
    e.path.endsWith("/context/reply-language.md")
      ? { value: `- ${MARK}日本語で書く。` }
      : { deny: "ENOENT" },
  );
  on("classic.PostToolBatch", () => ({}));
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

const carries = (context: readonly string[]): boolean =>
  context.some((c) => c.includes(MARK));

describe("a tool batch", () => {
  for (const tool of ["Skill", "Agent"]) {
    test(`that ran ${tool} gets the reply-language context`, async ($, on) => {
      expect(carries(await batch($, on, ["Read", tool]))).toBe(true);
    });
  }

  test("of other tools gets none", async ($, on) => {
    expect(carries(await batch($, on, ["Read", "Bash"]))).toBe(false);
  });
});

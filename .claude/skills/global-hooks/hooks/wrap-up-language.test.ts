import type { On } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

const WRAP_UP = "mcp__ccd_turn__wrap_up";

type Engine = { tool: { call: (input: never) => Promise<unknown> } };

// The engine beneath the mod: a tool answers "ran".
const engine = (on: On): void => {
  on("tool.call", () => ({ result: "ran" }));
};

// Whether a call with these arguments reached the tool beneath the mod.
const ran = async (
  $: Engine,
  args: Record<string, unknown>,
  tool = WRAP_UP,
): Promise<boolean> => {
  const called = await $.tool.call({ tool, ...args } as never);
  return JSON.stringify(called).includes('"result":"ran"');
};

describe("a wrap-up call", () => {
  test("is denied when its recap holds no Japanese", async ($, on) => {
    engine(on);
    expect(await ran($, { recap: "Committed abc123; nothing pushed." })).toBe(
      false,
    );
  });

  test("is denied when its ask holds no Japanese", async ($, on) => {
    engine(on);
    expect(
      await ran($, { recap: "変更をコミットした。", ask: "Push it now?" }),
    ).toBe(false);
  });

  test("runs when recap and ask are Japanese", async ($, on) => {
    engine(on);
    expect(
      await ran($, { recap: "変更をコミットした。", ask: "push しますか？" }),
    ).toBe(true);
  });

  // English words, sentences and code inside Japanese prose are the common shape.
  for (const recap of [
    "`wrap-up-language.ts` を足し、`mise run pre-commit` が通った。",
    'PR #366 を squash-merge した。Subject は "docs(instructions): ask once" のまま。',
    "テストを足した: `expect(await ran($, { recap })).toBe(false)`。",
    "The hook is in place. 残りは実セッションでの確認だけである。",
  ]) {
    test(`runs for Japanese mixed with English or code: ${recap}`, async ($, on) => {
      engine(on);
      expect(await ran($, { recap })).toBe(true);
    });
  }

  test("runs when it carries no recap or ask text", async ($, on) => {
    engine(on);
    expect(await ran($, { recap: "" })).toBe(true);
  });
});

describe("another tool", () => {
  test("runs with English arguments", async ($, on) => {
    engine(on);
    expect(await ran($, { recap: "English only" }, "mcp__other__tool")).toBe(
      true,
    );
  });
});

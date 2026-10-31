import type { On, PromptOrigin } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

const BUILT_IN = "mcp__Claude_Browser__navigate";
const CHROME = "mcp__claude-in-chrome__navigate";

type Engine = {
  prompt: { submit: (input: never) => Promise<unknown> };
  tool: { call: (input: never) => Promise<unknown> };
};

// The engine beneath the mod: a prompt enters, and a tool answers "ran".
const engine = (on: On): void => {
  on("prompt.submit", (_, e) => ({ text: e.text }));
  on("tool.call", () => ({ result: "ran" }));
};

const prompt = ($: Engine, kind: PromptOrigin["kind"]): Promise<unknown> =>
  $.prompt.submit({ text: "p", origin: { kind } } as never);

// Whether a call of the tool reached the tool beneath the mod.
const ran = async (
  $: Engine,
  tool: string,
  agentId?: string,
): Promise<boolean> => {
  const called = await $.tool.call({
    tool,
    url: "https://a.test",
    ...(agentId === undefined ? {} : { agentId }),
  } as never);
  return JSON.stringify(called).includes('"result":"ran"');
};

describe("a Chrome tool in a scheduled run", () => {
  test("is denied in the main conversation", async ($, on) => {
    engine(on);
    await prompt($, "scheduled-trigger");
    expect(await ran($, CHROME)).toBe(false);
  });

  for (const kind of ["composer", "bridge", "slack-ping"] as const) {
    test(`runs again once a person replies through ${kind}`, async ($, on) => {
      engine(on);
      await prompt($, "scheduled-trigger");
      await prompt($, kind);
      expect(await ran($, CHROME)).toBe(true);
    });
  }

  for (const kind of [
    "sdk",
    "task-notification",
    "peer",
    "peer-send-message",
    "projects-relay",
    "channel",
    "coordinator",
    "observer",
    "observer-activity",
    "auto-continuation",
    "unclassified",
    "plugin",
  ] as const) {
    test(`stays denied after a ${kind} prompt`, async ($, on) => {
      engine(on);
      await prompt($, "scheduled-trigger");
      await prompt($, kind);
      expect(await ran($, CHROME)).toBe(false);
    });
  }

  test("is denied inside a subagent", async ($, on) => {
    engine(on);
    await prompt($, "scheduled-trigger");
    expect(await ran($, CHROME, "a1")).toBe(false);
  });

  test("leaves the built-in browser alone", async ($, on) => {
    engine(on);
    await prompt($, "scheduled-trigger");
    expect(await ran($, BUILT_IN)).toBe(true);
  });

  test("leaves any other tool alone", async ($, on) => {
    engine(on);
    await prompt($, "scheduled-trigger");
    expect(await ran($, "WebFetch")).toBe(true);
  });
});

describe("a Chrome tool in a session a person started", () => {
  test("runs in the main conversation", async ($, on) => {
    engine(on);
    await prompt($, "composer");
    expect(await ran($, CHROME)).toBe(true);
  });

  test("runs after a /loop firing in that session", async ($, on) => {
    engine(on);
    await prompt($, "composer");
    await prompt($, "scheduled-trigger");
    expect(await ran($, CHROME)).toBe(true);
  });

  test("runs inside a subagent", async ($, on) => {
    engine(on);
    await prompt($, "composer");
    expect(await ran($, CHROME, "a1")).toBe(true);
  });
});

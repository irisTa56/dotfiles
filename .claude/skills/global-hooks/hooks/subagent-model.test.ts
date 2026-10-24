import type { AgentSpawnInput, On } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

// The engine beneath the mod: a spawn starts on the model it reaches it with,
// or on the parent's.
const engine = (on: On): void => {
  on("agent.spawn", (_, e) => ({
    model: e.model ?? e.parentModel,
    agentId: "a1",
  }));
};

// A spawn as the Agent tool makes it from the main session.
const spawn = (over: Partial<AgentSpawnInput>): AgentSpawnInput => ({
  tool_use_id: "t1",
  prompt: "p",
  description: "d",
  subagentType: "Explore",
  provider: { plugin: "engine", tier: "core" },
  parentModel: "claude-opus-5-5",
  background: false,
  fork: false,
  ...over,
});

describe("the Explore model", () => {
  test("is Sonnet where the spawn names none", async ($, on) => {
    engine(on);
    expect((await $.agent.spawn(spawn({}))).model).toBe("sonnet");
  });

  test("is the one the spawn names", async ($, on) => {
    engine(on);
    expect((await $.agent.spawn(spawn({ model: "haiku" }))).model).toBe(
      "haiku",
    );
  });

  test("is left alone for any other subagent", async ($, on) => {
    engine(on);
    expect(
      (await $.agent.spawn(spawn({ subagentType: "general-purpose" }))).model,
    ).toBe("claude-opus-5-5");
  });
});

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

  test("is left to its definition for an Explore that is not the built-in", async ($, on) => {
    engine(on);
    expect(
      (
        await $.agent.spawn(
          spawn({ provider: { plugin: "flag", tier: "user" } }),
        )
      ).model,
    ).toBe("claude-opus-5-5");
  });

  test("is left alone for any other subagent", async ($, on) => {
    engine(on);
    expect(
      (await $.agent.spawn(spawn({ subagentType: "general-purpose" }))).model,
    ).toBe("claude-opus-5-5");
  });
});

type Observed = {
  // Whether the call reached a person: the permission request the engine
  // raises for an ask, wherever in the call the plugin made it.
  asked: boolean;
  // The description of the Agent call as the permission decision sees it.
  description: unknown;
};

// Runs the call and reports whether it was put to a person and what the
// permission decision saw. The tool itself is stubbed, so no subagent starts,
// and the request is answered with a deny, so nothing waits on a person.
const run = async (
  $: { tool: { call: (input: never) => Promise<unknown> } },
  on: On,
  model: string | undefined,
  described = true,
): Promise<Observed> => {
  const seen: Observed = { asked: false, description: undefined };
  on("tool.check", { tool: "Agent" }, async (_, e, next) => {
    seen.description = (e.input as { description?: unknown }).description;
    return next(e);
  });
  on("classic.PermissionRequest", () => {
    seen.asked = true;
    return { decision: { behavior: "deny", message: "no person in a test" } };
  });
  on("tool.call", { tool: "Agent" }, () => ({ result: "ok" }));
  await $.tool
    .call({
      tool: "Agent",
      prompt: "p",
      ...(described ? { description: "Review the change" } : {}),
      subagent_type: "general-purpose",
      ...(model === undefined ? {} : { model }),
    } as never)
    .catch(() => undefined);
  return seen;
};

describe("an Agent call that names a model above Sonnet", () => {
  for (const model of ["opus", "fable", "claude-opus-5-5"]) {
    test(`asks the user about ${model}`, async ($, on) => {
      expect((await run($, on, model)).asked).toBe(true);
    });

    test(`puts ${model} in front of the description the prompt shows`, async ($, on) => {
      expect((await run($, on, model)).description).toBe(
        `Runs on ${model}: Review the change`,
      );
    });

    test(`shows only the model of a ${model} call that has no description`, async ($, on) => {
      expect((await run($, on, model, false)).description).toBe(
        `Runs on ${model}`,
      );
    });

    test(`names ${model} in the ask's reason`, async ($, on) => {
      on("tool.check", { tool: "Agent" }, () => ({ decision: "allow" }));
      const verdict = await $.tool.check({
        tool: "Agent",
        input: {
          prompt: "p",
          description: "Review the change",
          subagent_type: "general-purpose",
          model,
        },
      });
      expect(verdict.decision).toBe("ask");
      expect(verdict.reason).toContain(model);
    });
  }
});

describe("an Agent call that names Sonnet, Haiku or no model", () => {
  for (const model of ["sonnet", "haiku", "claude-haiku-5-5", undefined]) {
    test(`is not asked about for ${model ?? "no model"}`, async ($, on) => {
      const seen = await run($, on, model);
      expect(seen.asked).toBe(false);
      expect(seen.description).toBe("Review the change");
    });
  }
});

describe("a PreToolUse hook beneath the plugin", () => {
  test("still denies an Agent call that names opus", async ($, on) => {
    const seen = {
      hookRan: false,
      toolRan: false,
      outcome: undefined as unknown,
    };
    on("classic.PreToolUse", { tool: "Agent" }, () => {
      seen.hookRan = true;
      return { deny: "denied by the test hook" };
    });
    on("tool.call", { tool: "Agent" }, () => {
      seen.toolRan = true;
      return { result: "ok" };
    });
    seen.outcome = await $.tool
      .call({
        tool: "Agent",
        prompt: "p",
        description: "Review the change",
        subagent_type: "general-purpose",
        model: "opus",
      } as never)
      .catch((error: unknown) => error);
    expect(seen.hookRan).toBe(true);
    expect(seen.toolRan).toBe(false);
    expect(seen.outcome).toMatchObject({
      isError: true,
      text: expect.stringContaining("denied by the test hook"),
    });
  });
});

describe("a permission decision beneath the plugin that denies", () => {
  test("still denies an Agent call that names opus, without asking", async ($, on) => {
    const seen = { asked: false, toolRan: false };
    on("tool.check", { tool: "Agent" }, () => ({
      decision: "deny",
      reason: "denied beneath",
    }));
    on("classic.PermissionRequest", () => {
      seen.asked = true;
      return { decision: { behavior: "deny", message: "no person in a test" } };
    });
    on("tool.call", { tool: "Agent" }, () => {
      seen.toolRan = true;
      return { result: "ok" };
    });
    const outcome = await $.tool
      .call({
        tool: "Agent",
        prompt: "p",
        description: "Review the change",
        subagent_type: "general-purpose",
        model: "opus",
      } as never)
      .catch((error: unknown) => error);
    expect(seen.asked).toBe(false);
    expect(seen.toolRan).toBe(false);
    expect(outcome).toMatchObject({
      isError: true,
      text: expect.stringContaining("denied beneath"),
    });
  });
});

describe("a call to a tool other than Agent that carries model: opus", () => {
  test("is not asked about and reaches the permission decision unchanged", async ($, on) => {
    const tool = "mcp__other__spawn";
    const seen = {
      asked: false,
      description: undefined as unknown,
      model: undefined as unknown,
    };
    on("tool.check", { tool }, async (_, e, next) => {
      const input = e.input as { description?: unknown; model?: unknown };
      seen.description = input.description;
      seen.model = input.model;
      return next(e);
    });
    on("classic.PermissionRequest", () => {
      seen.asked = true;
      return { decision: { behavior: "deny", message: "no person in a test" } };
    });
    on("tool.call", { tool }, () => ({ result: "ok" }));
    await $.tool
      .call({
        tool,
        description: "Review the change",
        model: "opus",
      } as never)
      .catch(() => undefined);
    expect(seen.asked).toBe(false);
    expect(seen.description).toBe("Review the change");
    expect(seen.model).toBe("opus");
  });
});

describe("a permission decision beneath the plugin that asks with its own reason", () => {
  test("still asks, keeping that reason", async ($, on) => {
    on("tool.check", { tool: "Agent" }, () => ({
      decision: "ask",
      reason: "beneath has its own concern",
    }));
    const verdict = await $.tool.check({
      tool: "Agent",
      input: {
        prompt: "p",
        description: "Review the change",
        subagent_type: "general-purpose",
        model: "opus",
      },
    });
    expect(verdict.decision).toBe("ask");
    expect(verdict.reason).toContain("beneath has its own concern");
  });
});

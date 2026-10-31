import type { On } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

const CONNECTOR_TOOL = "mcp__7742f07e-8a59-4e28-8f2a-2a82893a8c56__any_name";

// The engine beneath the mod: its permission decision allows every call.
const engine = (on: On): void => {
  on("tool.check", () => ({ decision: "allow" }));
};

describe("a connector tool", () => {
  test("is asked about where its server does not declare it a read", async ($, on) => {
    engine(on);
    const verdict = await $.tool.check({ tool: CONNECTOR_TOOL, input: {} });
    expect(verdict.decision).toBe("ask");
  });

  test("keeps the engine's verdict where its server declares it a read", async ($, on) => {
    engine(on);
    const verdict = await $.tool.check({
      tool: CONNECTOR_TOOL,
      input: {},
      isReadOnly: true,
    } as never);
    expect(verdict.decision).toBe("allow");
  });

  test("keeps the engine's verdict where an allow rule settled it", async ($, on) => {
    on("tool.check", () => ({ decision: "allow", rule: "a rule" }));
    const verdict = await $.tool.check({ tool: CONNECTOR_TOOL, input: {} });
    expect(verdict.decision).toBe("allow");
  });

  test("is still denied where the engine denies it", async ($, on) => {
    on("tool.check", () => ({ decision: "deny", reason: "a deny rule" }));
    const verdict = await $.tool.check({ tool: CONNECTOR_TOOL, input: {} });
    expect(verdict.decision).toBe("deny");
    expect(verdict.reason).toBe("a deny rule");
  });

  test("is still denied where its server declares it a read", async ($, on) => {
    on("tool.check", () => ({ decision: "deny", reason: "a deny rule" }));
    const verdict = await $.tool.check({
      tool: CONNECTOR_TOOL,
      input: {},
      isReadOnly: true,
    } as never);
    expect(verdict.decision).toBe("deny");
  });

  test("becomes this plugin's ask where the engine would ask", async ($, on) => {
    on("tool.check", () => ({
      decision: "ask",
      reason: "the mode's sentence",
    }));
    const verdict = await $.tool.check({ tool: CONNECTOR_TOOL, input: {} });
    expect(verdict.decision).toBe("ask");
    expect(verdict.reason).toContain("connector");
    expect(verdict.plugin).toBeDefined();
  });
});

describe("a tool of a server registered by name", () => {
  for (const tool of ["mcp__basic-memory__bm_write", "Edit"]) {
    test(`keeps the engine's verdict for ${tool}`, async ($, on) => {
      engine(on);
      const verdict = await $.tool.check({ tool, input: {} });
      expect(verdict.decision).toBe("allow");
    });
  }
});

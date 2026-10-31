import type { On, ToolCheckResult } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

const CONNECTOR_TOOL = "mcp__7742f07e-8a59-4e28-8f2a-2a82893a8c56__any_name";

// The engine beneath the mod: its permission decision for a call.
const engine = (on: On, verdict: ToolCheckResult): void => {
  on("tool.check", () => verdict);
};

// What the engine answers for a tool no rule names, in each mode that was run.
const ASK: ToolCheckResult = { decision: "ask", reason: "the mode's sentence" };

describe("a connector tool", () => {
  test("becomes this plugin's ask where the engine would ask", async ($, on) => {
    engine(on, ASK);
    const verdict = await $.tool.check({ tool: CONNECTOR_TOOL, input: {} });
    expect(verdict.decision).toBe("ask");
    expect(verdict.reason).toContain("connector");
    expect(verdict.plugin).toBeDefined();
  });

  test("keeps the engine's ask where its server declares it a read", async ($, on) => {
    engine(on, ASK);
    const verdict = await $.tool.check({
      tool: CONNECTOR_TOOL,
      input: {},
      isReadOnly: true,
    } as never);
    expect(verdict.reason).toBe("the mode's sentence");
    expect(verdict.plugin).toBeUndefined();
  });

  test("stays allowed where an allow rule settled it", async ($, on) => {
    engine(on, { decision: "allow", rule: "a rule" });
    const verdict = await $.tool.check({ tool: CONNECTOR_TOOL, input: {} });
    expect(verdict.decision).toBe("allow");
  });

  test("stays allowed where anything else allowed it", async ($, on) => {
    engine(on, { decision: "allow" });
    const verdict = await $.tool.check({ tool: CONNECTOR_TOOL, input: {} });
    expect(verdict.decision).toBe("allow");
  });

  test("stays denied where the engine denies it", async ($, on) => {
    engine(on, { decision: "deny", reason: "a deny rule" });
    const verdict = await $.tool.check({ tool: CONNECTOR_TOOL, input: {} });
    expect(verdict.decision).toBe("deny");
    expect(verdict.reason).toBe("a deny rule");
  });

  test("stays denied where its server declares it a read", async ($, on) => {
    engine(on, { decision: "deny", reason: "a deny rule" });
    const verdict = await $.tool.check({
      tool: CONNECTOR_TOOL,
      input: {},
      isReadOnly: true,
    } as never);
    expect(verdict.decision).toBe("deny");
  });
});

describe("a tool of a server registered by name", () => {
  for (const tool of ["mcp__basic-memory__bm_write", "Edit"]) {
    test(`keeps the engine's ask for ${tool}`, async ($, on) => {
      engine(on, ASK);
      const verdict = await $.tool.check({ tool, input: {} });
      expect(verdict.reason).toBe("the mode's sentence");
      expect(verdict.plugin).toBeUndefined();
    });
  }
});

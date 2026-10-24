import type { On } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

type Observed = {
  // The engine's verdict on the call before the mode settles an ask.
  decision: string | undefined;
  // The description of the Agent call as the permission decision sees it, which
  // is what the permission prompt shows as its heading.
  description: unknown;
};

// Runs the call and reports what the engine decided and what the permission
// decision saw. The tool itself is stubbed, so no subagent starts.
const run = async (
  $: { tool: { call: (input: never) => Promise<unknown> } },
  on: On,
  model: string | undefined,
): Promise<Observed> => {
  const seen: Observed = { decision: undefined, description: undefined };
  on("tool.check", { tool: "Agent" }, async (_, e, next) => {
    seen.description = (e.input as { description?: unknown }).description;
    const verdict = await next(e);
    seen.decision = verdict.decision;
    return verdict;
  });
  on("tool.call", { tool: "Agent" }, () => ({ result: "ok" }));
  await $.tool.call({
    tool: "Agent",
    prompt: "p",
    description: "Review the change",
    subagent_type: "general-purpose",
    ...(model === undefined ? {} : { model }),
  } as never);
  return seen;
};

describe("an Agent call that names a model above Sonnet", () => {
  for (const model of ["opus", "fable", "claude-opus-5-5"]) {
    test(`asks the user about ${model}, the description leading with it`, async ($, on) => {
      const seen = await run($, on, model);
      expect(seen.decision).toBe("ask");
      expect(seen.description).toBe(`Runs on ${model}: Review the change`);
    });
  }
});

describe("an Agent call that names Sonnet, Haiku or no model", () => {
  for (const model of ["sonnet", "haiku", "claude-haiku-5-5", undefined]) {
    test(`is not asked about for ${model ?? "no model"}`, async ($, on) => {
      const seen = await run($, on, model);
      expect(seen.decision).not.toBe("ask");
      expect(seen.description).toBe("Review the change");
    });
  }
});

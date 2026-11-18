import type { On, PromptOrigin } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

type Engine = {
  prompt: {
    submit: (input: never) => Promise<{ context?: readonly string[] }>;
  };
};

// A line the context file holds, to tell it apart from other context.
const MARK = "ユーザーへの返答は";
// Context a hook above already attached on the way down.
const ABOVE = "above";

// The engine beneath the mod: the plugin's context file holds the line, and a
// prompt enters with the context that reached the bottom.
const engine = (on: On, readable = true): void => {
  on("fs.read", (_, e) =>
    readable && e.path.endsWith("/context/reply-language.md")
      ? { value: `- ${MARK}日本語で書く。` }
      : { deny: "ENOENT" },
  );
  on("prompt.submit", (_, e) => ({ text: e.text, context: e.context }));
};

const submit = ($: Engine, kind: PromptOrigin["kind"]) =>
  $.prompt.submit({ text: "p", origin: { kind }, context: [ABOVE] } as never);

const prompt = async (
  $: Engine,
  on: On,
  kind: PromptOrigin["kind"],
): Promise<readonly string[]> => {
  engine(on);
  return (await submit($, kind)).context ?? [];
};

const carries = (context: readonly string[]): number =>
  context.filter((c) => c.includes(MARK)).length;

describe("a prompt", () => {
  for (const kind of ["composer", "task-notification"] as const) {
    test(`from ${kind} gets the reply-language context once`, async ($, on) => {
      const context = await prompt($, on, kind);
      expect(carries(context)).toBe(1);
      expect(context).toContain(ABOVE);
    });
  }
});

describe("a prompt whose context file cannot be read", () => {
  test("enters without the context and the failure is reported", async ($, on) => {
    // Whatever the mod shows the person (toast, transcript line, notice).
    const reports: string[] = [];
    on("ui.toast", (_, e, next) => {
      reports.push(e.text);
      return next(e);
    });
    on("ui.log", (_, e, next) => {
      if (e.to !== "debug") reports.push(e.text);
      return next(e);
    });
    on("ui.notice", (_, e, next) => {
      if (e.text !== undefined) reports.push(e.text);
      return next(e);
    });
    engine(on, false);
    const context = (await submit($, "composer")).context ?? [];
    expect(carries(context)).toBe(0);
    expect(context).toContain(ABOVE);
    expect(reports.length).toBeGreaterThan(0);
  });
});

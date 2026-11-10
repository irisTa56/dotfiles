import type { On } from "claude-code";
import { expect, test } from "claude-code/testing";

const BAND = {
  plugin: "global-hooks",
  component: "AbovePrompt",
  surface: "desktop",
  requestId: "band",
  props: {
    hasSurvey: false,
    isWorking: false,
    maxRows: 10,
    bodyColumns: 80,
    scroll: { offset: 0, bodyRows: 10 },
    view: {},
  },
} as const;

// Stands for the engine: its usage figures, and a band of its own beneath the hook.
const engine = (on: On, tokens: number | undefined): void => {
  on("session.usage", () => ({
    value: { startedAt: 0, rateLimits: [], context: { tokens, window: 1e6 } },
  }));
  on("ui.render", ($, e) => {
    const { Text } = $.ui.resolve(e);

    return <Text>engine</Text>;
  });
};

test("under the threshold the band stays the engine's", async ($, on) => {
  engine(on, 299_999);

  const ui = await $.ui.mount(BAND);

  expect((await ui.find({ type: "Text" }))?.text).toBe("engine");
  await ui.unmount();
});

test("before any reading the band stays the engine's", async ($, on) => {
  engine(on, undefined);

  const ui = await $.ui.mount(BAND);

  expect((await ui.find({ type: "Text" }))?.text).toBe("engine");
  await ui.unmount();
});

test("at the threshold the band reminds, in thousands", async ($, on) => {
  engine(on, 300_000);

  const ui = await $.ui.mount(BAND);

  const row = await ui.find({ type: "Text", text: /\/compact/ });
  expect(row?.text).toContain("300k");
  await ui.unmount();
});

test("a survey keeps the band", async ($, on) => {
  engine(on, 500_000);

  const ui = await $.ui.mount({
    ...BAND,
    props: { ...BAND.props, hasSurvey: true },
  });

  expect((await ui.find({ type: "Text" }))?.text).toBe("engine");
  await ui.unmount();
});

test("a moved fill redraws the band, and other units do not", async ($, on) => {
  const redraws: string[] = [];
  on("ui.invalidate", (_, e) => {
    redraws.push(e.event);

    return { value: undefined };
  });
  on("session.measure", (_, e) => ({ changed: e.changed }));
  const context = { tokens: 1, window: 1e6 };

  await $.session.measure({ context, rateLimits: [], changed: ["cost"] });
  expect(redraws).toEqual([]);

  await $.session.measure({ context, rateLimits: [], changed: ["context"] });
  expect(redraws).toEqual(["ui.render"]);
});

test("a compaction redraws the band", async ($, on) => {
  const redraws: string[] = [];
  on("ui.invalidate", (_, e) => {
    redraws.push(e.event);

    return { value: undefined };
  });
  on("session.compact", (_, e) => ({ messages: e.messages }));

  await $.session.compact({
    trigger: "manual",
    messages: [{ role: "user", text: "summary", toolUses: [] }],
  });

  expect(redraws).toEqual(["ui.render"]);
});

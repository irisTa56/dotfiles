import type { On, PromptOrigin } from "claude-code";
import { describe, expect, mock, test } from "claude-code/testing";

const STORES =
  "/home/u/Library/Application Support/Claude/claude-code-sessions";
const STORE = `${STORES}/a/b/scheduled-tasks.json`;

type Engine = {
  prompt: { submit: (input: never) => Promise<unknown> };
  turn: { complete: (input: never) => Promise<unknown> };
};

type World = { banners: string[]; reports: string[] };

// The engine beneath the mod: Desktop's store holds the tasks, or a string where
// it can't be read, and the notify task is recorded, exiting with `postExit`, or
// failing to start where that is null.
// Whatever the module shows the person (toast, transcript line, notice) lands in
// `reports`.
const engine = (
  on: On,
  tasks: readonly unknown[] | string,
  postExit: number | null = 0,
): World => {
  const w: World = { banners: [], reports: [] };
  on("ui.toast", (_, e, next) => (w.reports.push(e.text), next(e)));
  on("ui.log", (_, e, next) => {
    if (e.to !== "debug") w.reports.push(e.text);

    return next(e);
  });
  on("ui.notice", (_, e, next) => {
    if (e.text !== undefined) w.reports.push(e.text);

    return next(e);
  });
  on("prompt.submit", (_, e) => ({ text: e.text }));
  on("turn.complete", (_, e) => ({ text: e.answer }));
  mock.env(on, { HOME: "/home/u" });
  on("fs.list", (_, e) => {
    const names: Record<string, string> = {
      [STORES]: "a",
      [`${STORES}/a`]: "b",
    };
    const name = names[e.path];

    return name === undefined
      ? { deny: "ENOENT" }
      : {
          value: [{ name, kind: "dir", size: 0, mtimeMs: 0, isLink: false }],
        };
  });
  on("fs.read", (_, e) =>
    e.path === STORE && typeof tasks !== "string"
      ? { value: JSON.stringify({ scheduledTasks: tasks }) }
      : { deny: "ENOENT" },
  );
  on("process.run", (_, e) => {
    // review-loop, beside this module, runs git on every turn.
    const isNotify =
      e.argv.slice(0, 6).join(" ") ===
      "mise -C /home/u run --skip-tools utils:notify";
    if (isNotify) w.banners.push(e.argv.join(" "));
    if (isNotify && postExit === null) return { deny: "ENOENT" };

    return {
      value: {
        exitCode: isNotify ? (postExit ?? 0) : 0,
        stdout: "",
        stderr: "",
        isStdoutTruncated: false,
        isStderrTruncated: false,
      },
    };
  });

  return w;
};

const dispatch = (id: string): string =>
  `<scheduled-task name="${id}" file="/t/${id}/SKILL.md">\nThis is an automated run.\n</scheduled-task>`;

const prompt = (
  $: Engine,
  kind: PromptOrigin["kind"],
  text: string,
): Promise<unknown> => $.prompt.submit({ text, origin: { kind } } as never);

const finish = (
  $: Engine,
  agentId?: string,
  reason = "answer",
): Promise<unknown> =>
  $.turn.complete({
    answer: reason === "answer" ? "done" : "",
    durationMs: 1,
    isAborted: false,
    reason,
    turnId: "t1",
    ...(agentId === undefined ? {} : { agentId }),
  } as never);

const ONE_TIME = { id: "check-a", fireAt: 1793439240000 };

describe("a one-time scheduled run", () => {
  test("posts a banner naming the task when its turn ends", async ($, on) => {
    const w = engine(on, [ONE_TIME]);
    await prompt($, "scheduled-trigger", dispatch("check-a"));
    await finish($);
    expect(w.banners.length).toBe(1);
    expect(w.banners[0]).toContain("check-a");
    expect(w.banners[0]).toContain("⏰");
  });

  test("posts once however many turns it takes", async ($, on) => {
    const w = engine(on, [ONE_TIME]);
    await prompt($, "scheduled-trigger", dispatch("check-a"));
    await finish($);
    await prompt($, "composer", "thanks, one more thing");
    await finish($);
    expect(w.banners.length).toBe(1);
  });

  test("posts nothing for a person's reply after its first turn failed", async ($, on) => {
    const w = engine(on, [ONE_TIME]);
    await prompt($, "scheduled-trigger", dispatch("check-a"));
    await finish($, undefined, "error");
    expect(w.banners).toEqual([]);
    await prompt($, "composer", "what happened?");
    await finish($);
    expect(w.banners).toEqual([]);
  });

  test("waits for the main turn, not a subagent's", async ($, on) => {
    const w = engine(on, [ONE_TIME]);
    await prompt($, "scheduled-trigger", dispatch("check-a"));
    await finish($, "a1");
    expect(w.banners).toEqual([]);
    await finish($);
    expect(w.banners.length).toBe(1);
  });
});

describe("a one-time scheduled run whose banner fails to post", () => {
  test("reports the failure and still completes the turn", async ($, on) => {
    const w = engine(on, [ONE_TIME], 1);
    await prompt($, "scheduled-trigger", dispatch("check-a"));
    const result = await finish($);
    expect(result).toEqual({ text: "done" });
    expect(w.banners.length).toBe(1);
    expect(w.reports.length).toBeGreaterThan(0);
  });

  test("reports a command that cannot start", async ($, on) => {
    const w = engine(on, [ONE_TIME], null);
    await prompt($, "scheduled-trigger", dispatch("check-a"));
    const result = await finish($);
    expect(result).toEqual({ text: "done" });
    expect(w.reports.length).toBeGreaterThan(0);
  });
});

describe("a scheduled run whose task cannot be looked up", () => {
  test("says so in a banner rather than staying silent", async ($, on) => {
    const w = engine(on, "unreadable");
    await prompt($, "scheduled-trigger", dispatch("check-a"));
    await finish($);
    expect(w.banners.length).toBe(1);
    expect(w.banners[0]).toContain("⚠️");
    expect(w.banners[0]).toContain("check-a");
  });
});

describe("a /loop firing in a person's session", () => {
  test("posts nothing", async ($, on) => {
    const w = engine(on, [ONE_TIME]);
    await prompt($, "composer", "hello");
    await prompt($, "scheduled-trigger", "check the deploy");
    await finish($);
    expect(w.banners).toEqual([]);
  });
});

describe("a scheduled run the store does not list", () => {
  test("says so in a banner", async ($, on) => {
    const w = engine(on, []);
    await prompt($, "scheduled-trigger", dispatch("check-a"));
    await finish($);
    expect(w.banners.length).toBe(1);
    expect(w.banners[0]).toContain("⚠️");
  });
});

describe("a recurring scheduled run", () => {
  test("posts nothing", async ($, on) => {
    const w = engine(on, [
      { id: "daily", fireAt: null, cronExpression: "0 9 * * *" },
    ]);
    await prompt($, "scheduled-trigger", dispatch("daily"));
    await finish($);
    expect(w.banners).toEqual([]);
  });
});

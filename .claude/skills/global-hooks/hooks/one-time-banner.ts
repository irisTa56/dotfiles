import type { EngineInterface, Register } from "claude-code";

const STORES = "Library/Application Support/Claude/claude-code-sessions";

// Desktop refuses any other characters in a task id, so the name needs no
// escaping in the AppleScript string below.
const TASK_NAME = /^<scheduled-task name="([a-z0-9_-]+)"/;

// Desktop's own store, under two opaque ids. It is read directly because auto
// mode refuses a mod's call of list_scheduled_tasks: the classifier has no model
// request to judge it against. The path and the shape aren't documented, which
// the warning banner below is there for.
const isOneTime = async (
  $: EngineInterface,
  taskId: string,
): Promise<boolean> => {
  const root = `${await $.env.get("HOME")}/${STORES}`;
  for (const account of await $.fs.list(root)) {
    for (const org of await $.fs.list(`${root}/${account.name}`)) {
      const store = await $.fs.read(
        `${root}/${account.name}/${org.name}/scheduled-tasks.json`,
      );
      const { scheduledTasks } = JSON.parse(store) as {
        scheduledTasks: readonly { id: string; fireAt: unknown }[];
      };
      const task = scheduledTasks.find((t) => t.id === taskId);
      if (task !== undefined) return task.fireAt != null;
    }
  }

  throw new Error(`no store lists ${taskId}`);
};

const post = async (
  $: EngineInterface,
  title: string,
  taskId: string,
): Promise<void> => {
  await $.process.run([
    "osascript",
    "-e",
    `display notification "${taskId}" with title "${title}" sound name "Glass"`,
  ]);
};

const REASONS = ["answer", "aborted", "refusal", "error"] as const;

// Desktop's own "Scheduled task completed" banner fires for every run and can't
// be told apart at a glance, so a one-time run's completion gets a banner of its
// own. A lookup that fails says so in a banner too: staying silent would look
// the same as a recurring run, and this repeats only until someone looks.
export const register: Register = (on) => {
  let taskId: string | undefined;

  on(
    "prompt.submit",
    { origin: { kind: "scheduled-trigger" } },
    (_, e, next) => {
      taskId = TASK_NAME.exec(e.text)?.[1];
      return next(e);
    },
  );

  // Every reason is named because review-loop holds this event's matcherless slot.
  on("turn.complete", { reason: REASONS }, async ($, e, next) => {
    if (e.agentId !== undefined) return next(e);

    const finished = taskId;
    taskId = undefined;
    // Desktop's own banner reads "Scheduled task failed" for the rest.
    if (finished === undefined || e.reason !== "answer") return next(e);

    await isOneTime($, finished).then(
      (oneTime) =>
        oneTime ? post($, "⏰ one-time 完了", finished) : undefined,
      () => post($, "⚠ one-time 判定失敗", finished),
    );

    return next(e);
  });
};

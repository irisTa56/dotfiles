import type { Register } from "claude-code";

const REMIND_AT = 300_000;

// Lowering the auto-compact window instead would compact there with no
// instructions, and a good summary depends on saying what comes next. So the
// window stays where it is and this band asks for a /compact by hand. It is a
// band because $.ui.status is one line per plugin, which review-loop holds.
export const register: Register = (on) => {
  on("session.measure", ($, e, next) => {
    if (e.changed.includes("context")) $.ui.invalidate("ui.render");

    return next(e);
  });

  on("session.compact", async ($, e, next) => {
    const compacted = await next(e);
    $.ui.invalidate("ui.render");

    return compacted;
  });

  on("ui.render", { component: "AbovePrompt" }, async ($, e, next) => {
    const { tokens = 0 } = (await $.session.usage()).context;
    if (tokens < REMIND_AT || e.props.hasSurvey) return next(e);

    const { Text } = $.ui.resolve(e);

    return (
      <Text>Context at {Math.floor(tokens / 1000)}k: time to /compact</Text>
    );
  });
};

import type { EngineInterface, Register } from "claude-code";

// The desktop app's ask, in its note on each side session's turn, to tell the
// user the outcome. A note rides on whatever prompt comes next, the user's own
// included, as well as on the turn the app starts for it.
const NOTE_ASK: readonly [RegExp, string] = [
  /, and mention the outcome to the user in one line when relevant\./g,
  ".",
];

// The same ask in the turn the app starts once a parent session's side
// sessions are idle, after its clause on steps their results unblock.
const EVENT_ASK: readonly [RegExp, string] = [
  /Otherwise tell the user the outcome in one or two lines \(with PR links if any\)\./g,
  "Otherwise, unless a note asks this session for something, end the turn with no text, not even a one-line status: the user reads each session's report in that session.",
];

const EVENT = /<side-sessions-event>/;

// Replaces the ask, and in the app's own turn, where a note is always present,
// logs an ask it no longer finds: the app's wording changed, and the relay is
// back until this file follows it.
const quiet = (
  $: EngineInterface,
  text: string,
  [ask, replacement]: readonly [RegExp, string],
): string => {
  const quieted = text.replace(ask, replacement);
  if (quieted === text && EVENT.test(text))
    $.ui.log(
      `side-session-quiet found no ${ask} in a side-sessions event, so the parent session may relay its side sessions' reports again`,
    );
  return quieted;
};

// Keeps a parent session from narrating its side sessions' progress, which the
// user already reads in those sessions. The app's ask sits at the end of every
// such prompt, where it outweighs the user-scoped CLAUDE.md rule saying the
// same. The turn still starts, so a step their results unblock goes on.
export const register: Register = (on) => {
  on(
    "prompt.submit",
    { text: /<side-sessions-event>|mention the outcome to the user/ },
    ($, e, next) => {
      const text = quiet($, e.text, NOTE_ASK);
      return next({
        ...e,
        text: EVENT.test(text) ? quiet($, text, EVENT_ASK) : text,
      });
    },
  );
};

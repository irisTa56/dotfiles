import type { EngineInterface, Register } from "claude-code";

// The desktop app's asks, in its note on each side session's turn, to tell the
// user the outcome, or that the session waits for the user, which its own row
// already shows. A note rides on whatever prompt comes next, the user's own
// included, as well as on the turn the app starts for it. The app ends a note
// in more than one way, some with no ask at all, so a note left untouched is no
// sign its wording changed.
const NOTE_ASK =
  /, and (?:mention the outcome to the user in one line when relevant|if it still waits, tell the user so in one line)\./g;

// The same ask in the turn the app starts once a parent session's side
// sessions are idle, after its clause on steps their results unblock.
const EVENT_ASK =
  /Otherwise tell the user the outcome in one or two lines \(with PR links if any\)\./g;
const EVENT_REPLACEMENT =
  "Otherwise, unless a note asks this session for something, end the turn with no text, not even a one-line status: the user reads each session's report in that session.";

const EVENT = /<side-sessions-event>/;

// Replaces the event's ask, and logs it when not found: every such event ends
// with it, so a miss means the app's wording changed, and the relay is back
// until this file follows it.
const quietEvent = ($: EngineInterface, text: string): string => {
  const quieted = text.replace(EVENT_ASK, EVENT_REPLACEMENT);
  if (quieted === text)
    $.ui.log(
      "side-session-quiet found no ask to replace in a side-sessions event, so the parent session may relay its side sessions' reports again",
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
    {
      text: /<side-sessions-event>|mention the outcome to the user|if it still waits, tell the user so/,
    },
    ($, e, next) => {
      const text = e.text.replace(NOTE_ASK, ".");
      return next({
        ...e,
        text: EVENT.test(text) ? quietEvent($, text) : text,
      });
    },
  );
};

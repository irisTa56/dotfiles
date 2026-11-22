import type { Register } from "claude-code";

// What the desktop app writes in the turn it starts once a session's side
// sessions are idle, asking for each one's outcome to be told to the user.
const RELAY: readonly [RegExp, string][] = [
  [/, and mention the outcome to the user in one line when relevant\./g, "."],
  [
    /Otherwise tell the user the outcome in one or two lines \(with PR links if any\)\./g,
    "Otherwise end the turn with no text, not even a one-line status: the user reads each session's report in that session.",
  ],
];

// Keeps a parent session from narrating its side sessions' progress, which the
// user already reads in those sessions. The app's own ask sits at the end of
// every such turn, where it outweighs the user-scoped CLAUDE.md rule saying
// the same. The turn still starts, so a step its results unblock goes on.
export const register: Register = (on) => {
  on("prompt.submit", { text: /<side-sessions-event>/ }, ($, e, next) => {
    let text = e.text;
    for (const [ask, quiet] of RELAY) {
      const quieted = text.replace(ask, quiet);
      // The app's wording changed: the relay is back until RELAY follows it.
      if (quieted === text)
        $.ui.log(
          `side-session-quiet found no ${ask} in a side-sessions event, so the parent session may relay its side sessions' reports again`,
        );
      text = quieted;
    }
    return next({ ...e, text });
  });
};

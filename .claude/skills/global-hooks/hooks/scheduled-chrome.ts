import type { PromptOrigin, Register } from "claude-code";

const CHROME_TOOL = /^mcp__claude-in-chrome__/;

// The origins of a prompt a person sent.
const PERSON: readonly PromptOrigin["kind"][] = [
  "composer",
  "bridge",
  "slack-ping",
];

// Keeps the user's own Chrome out of a run nobody is watching, in its main
// conversation and in its subagents alike, since a run reads third-party pages.
// The built-in browser stays, by the user's choice. A person replying to the run
// is watching it again.
export const register: Register = (on) => {
  let scheduled: boolean | undefined;

  on("prompt.submit", (_, e, next) => {
    // Only the first prompt says so: a /loop firing in a person's session has
    // the same origin as a scheduled task's.
    if (scheduled === undefined)
      scheduled = e.origin.kind === "scheduled-trigger";
    else if (PERSON.includes(e.origin.kind)) scheduled = false;
    return next(e);
  });

  on("tool.call", { tool: CHROME_TOOL }, (_, e, next) =>
    scheduled
      ? {
          deny: "Blocked: a scheduled run may not use the user's Chrome. Read the page with WebFetch or the built-in browser, or report it as unread and leave it for a person.",
        }
      : next(e),
  );
};

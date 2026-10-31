import type { PromptOrigin, Register } from "claude-code";

const BROWSER_TOOL = /^mcp__(Claude_Browser|claude-in-chrome)__/;

// The origins of a prompt a person sent.
const PERSON: readonly PromptOrigin["kind"][] = [
  "composer",
  "bridge",
  "slack-ping",
];

// Keeps both browsers out of a run nobody is watching, in its main conversation
// and in its subagents alike. These browsers can carry the user's logins, and a
// run reads third-party pages. A person replying to the run is watching it again.
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

  on("tool.call", { tool: BROWSER_TOOL }, (_, e, next) =>
    scheduled
      ? {
          deny: "Blocked: a scheduled run has no browser. Read the page with WebFetch, or report it as unread and leave it for a person.",
        }
      : next(e),
  );
};

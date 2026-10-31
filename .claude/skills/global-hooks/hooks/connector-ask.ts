import type { Register } from "claude-code";

// A connector's tool, whose server a Desktop session names by a UUID.
const CONNECTOR_TOOL = /^mcp__[0-9a-f-]{36}__/;

// Lays a base under the permission rules: a connector acts on outside services
// under the user's account, where a wrong call cannot be taken back, so where the
// engine would ask, a person answers rather than auto mode's classifier. The
// engine asks for any tool no rule names, so a tool added later is covered without
// anyone listing it, and a deny or allow rule sits on top. A read its server
// declares keeps the mode's own ask, since a prompt on every read would teach the
// user to approve without looking.
export const register: Register = (on) => {
  on("tool.check", { tool: CONNECTOR_TOOL }, async (_, e, next) => {
    const verdict = await next(e);
    return verdict.decision === "ask" && !e.isReadOnly
      ? { decision: "ask", reason: "connector tool that may need attention" }
      : verdict;
  });
};

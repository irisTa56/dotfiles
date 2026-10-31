import type { Register } from "claude-code";

// A connector's tool, whose server a Desktop session names by a UUID.
const CONNECTOR_TOOL = /^mcp__[0-9a-f-]{36}__/;

// Makes a person the gate for a connector, which acts on outside services under
// the user's account, where a wrong call cannot be taken back, and which auto
// mode's classifier would otherwise settle with no prompt. It asks by default,
// so a tool added later is covered without anyone listing it, and lets through
// what the server declares a read, since a prompt on every read would teach the
// user to approve without looking. Only that declaration exempts a tool: an allow
// rule beneath does not settle the call. A deny beneath stands. An ask beneath is
// answered anew rather than handed up, since only an ask a plugin made is kept
// from the classifier.
export const register: Register = (on) => {
  on("tool.check", { tool: CONNECTOR_TOOL }, async (_, e, next) => {
    const verdict = await next(e);
    return e.isReadOnly || verdict.decision === "deny"
      ? verdict
      : { decision: "ask", reason: "connector tool that may need attention" };
  });
};

import type { Register } from "claude-code";

// A connector's tool, whose server a Desktop session names by a UUID.
const CONNECTOR_TOOL = /^mcp__[0-9a-f-]{36}__/;

// Lays a base under the permission rules: a connector acts on outside services
// under the user's account, where a wrong call cannot be taken back, so even in
// auto mode a person decides it unless something explicit already has. The
// server declaring a read counts, since a prompt on every read would teach the
// user to approve without looking, and so do the user's deny and allow rules,
// which sit on top of the base. Asking by default covers a tool added later
// without anyone listing it. This ask replaces one beneath rather than handing
// that up, since only an ask a plugin made is kept from the classifier.
export const register: Register = (on) => {
  on("tool.check", { tool: CONNECTOR_TOOL }, async (_, e, next) => {
    const verdict = await next(e);
    const explicit =
      verdict.decision === "deny" ||
      (verdict.decision === "allow" && verdict.rule !== undefined);
    return e.isReadOnly || explicit
      ? verdict
      : { decision: "ask", reason: "connector tool that may need attention" };
  });
};

import type { On, ToolCheckResult } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

// What Claude is told when the call is blocked.
const BLOCKED = `\
Blocked: sed on PATH here is GNU sed, whose -i takes no separate suffix
argument, so the empty string after -i is read as the script or as a file
name and the call fails. Write \`sed -i 's/.../.../' file\`, or use the Edit
tool where the edit is not mechanical.
`;

// The engine beneath the mod, settling the call as a rule that allows it would.
const ALLOWED: ToolCheckResult = { decision: "allow", rule: "Bash(sed:*)" };

// The sed on PATH: GNU's or BSD's, told apart by whether `sed --version`
// succeeds, or one that cannot be started.
type Sed = "gnu" | "bsd" | "missing";

const engine = (on: On, sed: Sed): void => {
  on("tool.check", { tool: "Bash" }, () => ALLOWED);
  on("process.run", (_, e) =>
    e.argv[0] !== "sed" || sed === "missing"
      ? { deny: `${e.argv[0]}: command not found` }
      : {
          value: {
            exitCode: sed === "gnu" ? 0 : 1,
            stdout: "",
            stderr: "",
            isStdoutTruncated: false,
            isStderrTruncated: false,
          },
        },
  );
};

// The verdict on a Bash call of `command`.
const check = async (
  $: { tool: { check: (input: never) => Promise<ToolCheckResult> } },
  command: string,
): Promise<ToolCheckResult> =>
  $.tool.check({
    tool: "Bash",
    input: { command, description: "Runs a command" },
  } as never);

describe("block-bsd-sed-inplace", () => {
  // GNU sed reads the empty suffix as the script or a file name, so the call is
  // blocked, over a rule that allows it.
  test("blocks an empty suffix under GNU sed", async ($, on) => {
    engine(on, "gnu");
    expect(await check($, "sed -i '' 's/a/b/' file")).toMatchObject({
      decision: "deny",
      reason: BLOCKED,
    });
  });

  // BSD sed takes the empty string as the suffix, the form it documents, so the
  // call passes.
  test("passes an empty suffix under BSD sed", async ($, on) => {
    engine(on, "bsd");
    expect(await check($, "sed -i '' 's/a/b/' file")).toEqual(ALLOWED);
  });

  // GNU sed's own form, with no separate suffix, passes.
  test("passes GNU sed's own form", async ($, on) => {
    engine(on, "gnu");
    expect(await check($, "sed -i 's/a/b/' file")).toEqual(ALLOWED);
  });

  // Where sed cannot be started there is no GNU sed to fail, and the call
  // passes.
  test("passes where sed cannot be started", async ($, on) => {
    engine(on, "missing");
    expect(await check($, "sed -i '' 's/a/b/' file")).toEqual(ALLOWED);
  });

  // A call with no command to judge goes ahead.
  for (const input of [{}, { command: 5 }, null]) {
    test(`passes a call whose input is ${JSON.stringify(input)}`, async ($, on) => {
      engine(on, "gnu");
      expect(await $.tool.check({ tool: "Bash", input })).toEqual(ALLOWED);
    });
  }

  // A failure of the hook's own blocks the call, naming the hook and the cause,
  // since the engine would otherwise skip the hook and let the call through.
  test("blocks the call where the hook itself fails", async ($, on) => {
    on("tool.check", { tool: "Bash" }, () => ALLOWED);
    on("process.run", () => ({ value: null as never }));
    const verdict = await check($, "sed -i '' 's/a/b/' file");
    expect(verdict.decision).toBe("deny");
    expect(verdict.reason).toContain("block-bsd-sed-inplace: TypeError");
  });

  // Where the shell would run a bare sed with -i and an empty suffix, the call
  // is blocked: at the start of any line, after a separator or a space, with
  // options before -i, and with either quote. The same text after a space inside
  // a quoted argument is caught too.
  for (const command of [
    `sed -i "" 's/a/b/' file`,
    "sed -E -i '' 's/a/b/' file",
    "sed  -n -E  -i  '' 's/a/b/' file",
    "cd dir;sed -i '' 's/a/b/' file",
    "true&&sed -i '' 's/a/b/' file",
    "(sed -i '' 's/a/b/' file)",
    "echo start\nsed -i '' 's/a/b/' file",
    "xargs\tsed -i '' 's/a/b/'",
    "git commit -m 'run sed -i '' here'",
  ]) {
    test(`blocks ${JSON.stringify(command)}`, async ($, on) => {
      engine(on, "gnu");
      expect(await check($, command)).toMatchObject({
        decision: "deny",
        reason: BLOCKED,
      });
    });
  }

  // Where the empty string is no separate suffix, or the sed is not a bare one,
  // the call passes, as it does beside a Unicode space, which the shell does not
  // split words on.
  for (const command of [
    "sed -i.bak 's/a/b/' file",
    "sed -i'' 's/a/b/' file",
    "sed -e '' -i file",
    "/usr/bin/sed -i '' 's/a/b/' file",
    "gsed -i '' 's/a/b/' file",
    "echo x |sed -i '' file",
    "git commit -m 'sed -i '' is BSD'",
    "sed --in-place '' file",
    "echo x sed -i '' 's/a/b/' file",
    "",
  ]) {
    test(`passes ${JSON.stringify(command)}`, async ($, on) => {
      engine(on, "gnu");
      expect(await check($, command)).toEqual(ALLOWED);
    });
  }
});

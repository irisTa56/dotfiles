import type { On, ToolCheckResult } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

// What Claude is told when the call is blocked.
const BLOCKED = `\
Blocked: stat on PATH here is GNU stat, whose -f reports the file system and
takes no format, so the format is read as a file name and the call fails.
Write \`stat -c '%Y %s %n' file\` for mtime, size and name, or
\`stat -c '%y' file\` for a readable mtime; on macOS, /usr/bin/stat is BSD stat.
Where the command only quotes the form in text, such as a commit message,
reword that text.
`;

// The engine beneath the mod, settling the call as a rule that allows it would.
const ALLOWED: ToolCheckResult = { decision: "allow", rule: "Bash(stat:*)" };

// The stat on PATH: GNU's or BSD's, told apart by whether `stat --version`
// succeeds, or one that cannot be started.
type Stat = "gnu" | "bsd" | "missing";

const engine = (on: On, stat: Stat): void => {
  on("tool.check", { tool: "Bash" }, () => ALLOWED);
  on("process.run", (_, e) =>
    e.argv[0] !== "stat" || stat === "missing"
      ? { deny: `${e.argv[0]}: command not found` }
      : {
          value: {
            exitCode: stat === "gnu" ? 0 : 1,
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

describe("block-bsd-stat-format", () => {
  // GNU stat reads the format as a file name, so the call is blocked, over a
  // rule that allows it.
  test("blocks a format under GNU stat", async ($, on) => {
    engine(on, "gnu");
    expect(await check($, "stat -f %z file")).toMatchObject({
      decision: "deny",
      reason: BLOCKED,
    });
  });

  // BSD stat takes the format after -f, the form it documents, so the call
  // passes.
  test("passes a format under BSD stat", async ($, on) => {
    engine(on, "bsd");
    expect(await check($, "stat -f %z file")).toEqual(ALLOWED);
  });

  // GNU stat's own form, the format after -c, passes.
  test("passes GNU stat's own form", async ($, on) => {
    engine(on, "gnu");
    expect(await check($, "stat -c %s file")).toEqual(ALLOWED);
  });

  // Where stat cannot be started there is no GNU stat to fail, and the call
  // passes.
  test("passes where stat cannot be started", async ($, on) => {
    engine(on, "missing");
    expect(await check($, "stat -f %z file")).toEqual(ALLOWED);
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
    const verdict = await check($, "stat -f %z file");
    expect(verdict.decision).toBe("deny");
    expect(verdict.reason).toContain("block-bsd-stat-format: TypeError");
  });

  // Where the shell would run a bare stat with -f and a format, the call is
  // blocked: the format quoted or not, attached to -f or not, or with text
  // before its first %; at the start of any line, after a separator or a space,
  // and with options before -f. The same text after a space inside a quoted
  // argument is caught too.
  for (const command of [
    "stat -f '%Sm %N' file",
    `stat -f "%z" file`,
    "stat -f%z file",
    "stat -f'%z' file",
    "stat -f 'size=%z' file",
    "stat -L -f %z file",
    "stat  -L  -x  -f  %z file",
    "cd dir;stat -f %z file",
    "true&&stat -f %z file",
    "(stat -f %z file)",
    "echo start\nstat -f %z file",
    "xargs\tstat -f %z",
    "git commit -m 'run stat -f %z here'",
  ]) {
    test(`blocks ${JSON.stringify(command)}`, async ($, on) => {
      engine(on, "gnu");
      expect(await check($, command)).toMatchObject({
        decision: "deny",
        reason: BLOCKED,
      });
    });
  }

  // Where the word after -f is a path or an option rather than a format, or the
  // stat is not a bare one, the call passes: GNU stat's own -f takes a path, and
  // its format follows -c. A Unicode space, which the shell does not split words
  // on, is no space before the word either.
  for (const command of [
    "stat -f /",
    "stat -f -c%T /",
    "stat -f -c %T /",
    "stat -fc%T /",
    "stat -f",
    "/usr/bin/stat -f %z file",
    "gstat -f %z file",
    "echo x |stat -f %z file",
    "git commit -m 'stat -f %z is BSD'",
    "echo x stat -f %z file",
    "",
  ]) {
    test(`passes ${JSON.stringify(command)}`, async ($, on) => {
      engine(on, "gnu");
      expect(await check($, command)).toEqual(ALLOWED);
    });
  }
});

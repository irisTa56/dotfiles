import type { On, ToolCheckResult } from "claude-code";
import { describe, expect, test } from "claude-code/testing";

// What Claude is told when the call is blocked.
const BLOCKED = `\
Blocked: the git merge/rebase/pull/commit is piped, so && tests the last pipe
stage and the push would run even if that step failed. Drop the pipe, or push
in a separate command after checking the result.
`;

// The engine beneath the mod, settling the call as a rule that allows it would:
// `Bash(git push:*)` matches what the hook blocks.
const ALLOWED: ToolCheckResult = {
  decision: "allow",
  rule: "Bash(git push:*)",
};

const allowed = (on: On): void => {
  on("tool.check", { tool: "Bash" }, () => ALLOWED);
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

describe("block-piped-git-push", () => {
  // && tests only the last stage of the pipe, so the push would follow a failed merge.
  test("blocks a push after a piped merge, over a rule that allows it", async ($, on) => {
    allowed(on);
    expect(
      await check($, "git merge --ff-only x | tail -1 && git push"),
    ).toMatchObject({ decision: "deny", reason: BLOCKED });
  });

  // Each of the four git steps, piped and followed by && and a push, is
  // blocked: with words or none between git and the step, between the step and
  // the pipe, between the pipe and &&, between && and git, and between git and
  // push, with several spaces in each gap, with more pipes before the &&, and
  // wherever the chain sits on the line, inside a quoted argument too, where the
  // gap after the pipe can be empty.
  for (const command of [
    "git rebase main | tail -1 && git push",
    "git pull | tail -1 && git push",
    "git commit -m x | tail -1 && git push",
    "git -C repo merge x | tail -1 && git push",
    "git pull| tail -1 && git push",
    "git merge x | tail -1 &&git push",
    "git merge x | tail -1 && cd .. && git push",
    "git merge x | tail -1 && git -C repo push origin main",
    "git  merge  x  |  tail -1  &&  git  push",
    "git pull | tee log | tail -1 && git push",
    "cd repo && git merge x | tail -1 && git push",
    "echo 'git merge x | tail -1 && git push'",
    "echo 'git pull|&&git push'",
  ]) {
    test(`blocks ${JSON.stringify(command)}`, async ($, on) => {
      allowed(on);
      expect(await check($, command)).toMatchObject({
        decision: "deny",
        reason: BLOCKED,
      });
    });
  }

  // Where no piped git step is followed by && and a push on the same side of a
  // ;, the call passes. A step counts only before any pipe after its git, so not
  // as an argument to a later stage; a step or a push counts only after a space,
  // so not inside a word such as fix/merge or git-push; and a single & runs the
  // push whatever the pipe gave.
  for (const command of [
    "git merge x && git push",
    "git merge x | tail -1; git push",
    "git merge x | tail -1 || git push",
    "git merge x | tail -1 & git push",
    "git checkout fix/merge | cat && git push",
    "git merge x | tail -1 && echo git-push",
    "git merge x | tail -1 && git status",
    "git log | head -1 && git push",
    "git merge x; echo y | tail -1 && git push",
    "git log | grep merge | tail -1 && git push",
    "git merge x | tail -1; true && git push",
    "git merge x | tail -1 && git fetch; git push",
    "",
  ]) {
    test(`passes ${JSON.stringify(command)}`, async ($, on) => {
      allowed(on);
      expect(await check($, command)).toEqual(ALLOWED);
    });
  }

  // A long chain with no push anywhere passes, and is judged promptly: the
  // judging must not backtrack over the many && segments until the hook's
  // budget runs out and its failure handler blocks the call.
  test("passes a long chain with no push, promptly", async ($, on) => {
    allowed(on);
    const command =
      "git merge b | tail -1 && git status && ".repeat(200) + "true";
    const start = performance.now();
    const verdict = await check($, command);
    const elapsed = performance.now() - start;
    expect(verdict).toEqual(ALLOWED);
    expect(elapsed).toBeLessThan(1000);
  });

  // A line that mentions pipefail is taken to set it, under which && tests
  // every stage, so it passes. Another line of the same command is still judged.
  test("skips a line that mentions pipefail", async ($, on) => {
    allowed(on);
    expect(
      await check($, "set -o pipefail; git merge x | tail -1 && git push"),
    ).toEqual(ALLOWED);
    expect(
      await check($, "set -o pipefail\ngit merge x | tail -1 && git push"),
    ).toMatchObject({ decision: "deny", reason: BLOCKED });
  });

  // The command is judged a line at a time, so a chain split across lines
  // passes, as do pieces of one on separate lines.
  for (const command of [
    "git merge x | tail -1 &&\ngit push",
    "git merge x\necho y | tail -1 && git push",
  ]) {
    test(`judges each line of ${JSON.stringify(command)} on its own`, async ($, on) => {
      allowed(on);
      expect(await check($, command)).toEqual(ALLOWED);
    });
  }

  // A call with no command to judge goes ahead.
  for (const input of [{}, { command: 5 }, null]) {
    test(`passes a call whose input is ${JSON.stringify(input)}`, async ($, on) => {
      allowed(on);
      expect(await $.tool.check({ tool: "Bash", input })).toEqual(ALLOWED);
    });
  }

  // The hook judges Bash calls alone.
  test("leaves another tool's command alone", async ($, on) => {
    on("tool.check", () => ALLOWED);
    expect(
      await $.tool.check({
        tool: "mcp__other__run",
        input: { command: "git merge x | tail -1 && git push" },
      }),
    ).toEqual(ALLOWED);
  });
});

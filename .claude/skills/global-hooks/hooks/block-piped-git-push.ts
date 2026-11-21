import type { Block } from "./block-bash";

// A git step that, piped, leaves && testing only the last stage of the pipe.
const STEP = / (?:merge|rebase|pull|commit)/;

// What Claude is told when the call is blocked.
const BLOCKED = `\
Blocked: the git merge/rebase/pull/commit is piped, so && tests the last pipe
stage and the push would run even if that step failed. Drop the pipe, or push
in a separate command after checking the result.
`;

// Whether `stage`, text between pipes, runs a git step: git, then after it a
// space and the step's name.
const runsAGitStep = (stage: string): boolean => {
  const git = stage.indexOf("git");
  return git >= 0 && STEP.test(stage.slice(git + 3));
};

// Whether `rest` chains a push with &&: &&, then git, then a space and push.
const chainsAPush = (rest: string): boolean => {
  const and = rest.indexOf("&&");
  const git = and < 0 ? -1 : rest.indexOf("git", and + 2);
  return git >= 0 && rest.includes(" push", git + 3);
};

// Whether a command with no ; pipes a git step into a stage that && then
// follows with a git push. The first stage that runs a git step leaves the most
// after its pipe, so it alone is judged. Each search runs once, so the time
// stays linear in the command's length, where one regular expression with these
// gaps backtracks for seconds on a long generated line.
const pushesAfterAPipeIn = (part: string): boolean => {
  const stages = part.split("|");
  const step = stages.slice(0, -1).findIndex(runsAGitStep);
  return step >= 0 && chainsAPush(stages.slice(step + 1).join("|"));
};

// Whether a line of the command pushes after a piped git step, chained with &&
// on the same side of a ;. A line that mentions pipefail is taken to set it, and
// is skipped.
const pushesAfterAPipe = (command: string): boolean =>
  command
    .split("\n")
    .some(
      (line) =>
        !line.includes("pipefail") && line.split(";").some(pushesAfterAPipeIn),
    );

// Blocks a `git push` chained with && after a piped git merge, rebase, pull or
// commit: without pipefail, && tests the last stage of the pipe, so
// `git merge --ff-only x | tail -1 && git push` pushes even when the merge
// failed.
export const blockPipedGitPush: Block = {
  name: "block-piped-git-push",
  matches: pushesAfterAPipe,
  reason: BLOCKED,
};

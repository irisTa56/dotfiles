import type { Block } from "./block-bash";

// A git merge, rebase, pull or commit piped into a stage that && then follows
// with a git push.
const PIPED_THEN_PUSH =
  /git[^|;]* (merge|rebase|pull|commit)[^|;]*\|[^;]*&&[^;]*git[^;]* push/;

// What Claude is told when the call is blocked.
const BLOCKED = `\
Blocked: the git merge/rebase/pull/commit is piped, so && tests the last pipe
stage and the push would run even if that step failed. Drop the pipe, or push
in a separate command after checking the result.
`;

// Whether a line of the command pushes after a piped git step, chained with &&.
// A line that mentions pipefail is taken to set it, and is skipped.
const pushesAfterAPipe = (command: string): boolean =>
  command
    .split("\n")
    .some((line) => !line.includes("pipefail") && PIPED_THEN_PUSH.test(line));

// Blocks a `git push` chained with && after a piped git merge, rebase, pull or
// commit: without pipefail, && tests the last stage of the pipe, so
// `git merge --ff-only x | tail -1 && git push` pushes even when the merge
// failed.
export const blockPipedGitPush: Block = {
  name: "block-piped-git-push",
  matches: pushesAfterAPipe,
  reason: BLOCKED,
};

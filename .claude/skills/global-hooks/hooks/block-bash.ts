import type { EngineInterface, Register } from "claude-code";
import { blockBsdSedInplace } from "./block-bsd-sed-inplace";
import { blockBsdStatFormat } from "./block-bsd-stat-format";
import { blockPipedGitPush } from "./block-piped-git-push";

// A form of Bash command that one of the hooks below blocks.
export type Block = {
  // What the hook is called, in the reason given when it fails.
  name: string;
  // Whether the command is one the hook blocks.
  matches: (command: string) => boolean;
  // What Claude is told when the call is blocked.
  reason: string;
  // A tool that comes as GNU's or BSD's, where the hook blocks a BSD form only
  // while the one on PATH is GNU's.
  gnu?: string;
};

const BLOCKS: readonly Block[] = [
  blockBsdSedInplace,
  blockBsdStatFormat,
  blockPipedGitPush,
];

// Whether the tool on PATH is GNU's, the one of GNU's and BSD's that knows
// `--version`. A tool that cannot be started or fails is not. A result of an
// unexpected shape throws, failing the hook.
function isGnu($: EngineInterface, tool: string): Promise<boolean> {
  return $.process.run([tool, "--version"]).then(
    (version) => version.exitCode === 0,
    () => false,
  );
}

// Whether `block` blocks the command, a throw naming the hook it failed in.
async function blocks(
  $: EngineInterface,
  block: Block,
  command: string,
): Promise<boolean> {
  try {
    return (
      block.matches(command) &&
      (block.gnu === undefined || (await isGnu($, block.gnu)))
    );
  } catch (error) {
    throw new Error(`${block.name}: ${String(error)}`);
  }
}

// Blocks a Bash call that any of BLOCKS matches, over whatever the rules and the
// mode settled, an allow rule that matches it included.
//
// A call with no command to judge goes ahead, as does one where the tool to ask
// cannot be started or fails. Anything else that fails, a throw or an overrun
// budget, blocks the call naming the hook and the cause, since the engine would
// otherwise skip the hook and let the call through unjudged.
//
// The hooks share one registration and one `.catch`: where this module
// registered a second `tool.check` hook with a `.catch` of its own, that hook's
// failure was answered by the first one's `.catch`.
export const register: Register = (on) => {
  on("tool.check", { tool: "Bash" }, async ($, e, next) => {
    const { command } = (e.input ?? {}) as { command?: unknown };
    if (typeof command === "string")
      for (const block of BLOCKS)
        if (await blocks($, block, command))
          return { decision: "deny", reason: block.reason };
    return next(e);
  }).catch((_, __, next) => ({
    decision: "deny",
    reason: `A hook that blocks Bash calls failed (${
      next.error.message ??
      `${BLOCKS.map((block) => block.name).join(", ")}: ${next.error.kind}`
    }), so the call is blocked until the hook is fixed.`,
  }));
};

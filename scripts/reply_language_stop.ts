#!/usr/bin/env node
// Stop command hook, registered in .config/claude/settings.json and linked as
// ~/.local/bin/reply_language_stop; node strips its types. It sends a final
// reply in English back once to be rewritten in Japanese, as global-hooks'
// reply-language-judge.ts judges it. It is a settings hook rather than a
// classic.Stop hook in the plugin's module because a managed plugin may
// continue that event past the user tier, where the module's hook never runs;
// a settings hook runs at the core tier, which such a continuation still
// reaches.
//
// Each stop is a process of its own, so what one stop leaves for the next is
// kept in a file per session.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  judgeStop,
  type StopInput,
} from "../.claude/skills/global-hooks/hooks/reply-language-judge.ts";

const input: StopInput & { session_id: string } = JSON.parse(
  readFileSync(0, "utf8"),
);

const dir = join(tmpdir(), "claude-reply-language");
const state = join(dir, input.session_id.replace(/[^\w-]/g, "_"));

let rewriteAsked = false;
try {
  rewriteAsked = readFileSync(state, "utf8") === "1";
} catch {
  // No earlier stop in this session.
}

const verdict = judgeStop(input, rewriteAsked);
mkdirSync(dir, { recursive: true });
writeFileSync(state, verdict.rewriteAsked ? "1" : "0");

if (verdict.block !== undefined) {
  process.stdout.write(
    JSON.stringify({ decision: "block", reason: verdict.block }),
  );
}

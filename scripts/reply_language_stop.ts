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
  fileState,
  handleStop,
} from "../.claude/skills/global-hooks/hooks/reply-language-judge.ts";

process.stdout.write(
  handleStop(
    readFileSync(0, "utf8"),
    fileState(join(tmpdir(), "claude-reply-language"), {
      read: (path) => readFileSync(path, "utf8"),
      write: (path, text) => writeFileSync(path, text),
      mkdir: (path) => mkdirSync(path, { recursive: true }),
    }),
  ),
);

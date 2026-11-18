// Stop command hook, registered in hooks.json and run by node, which strips
// the types: sends a final reply in English back once to be rewritten in
// Japanese, as reply-language-judge.ts judges it. It is a command hook rather
// than a classic.Stop hook in the module because a managed plugin may continue
// that event past the user tier, where the module's hook never runs.
//
// Each stop is a process of its own, so what one stop leaves for the next is
// kept in a file per session.

// The plugin's type check has neither Node's types nor .ts import extensions,
// so the names these three imports bring in go unchecked; the rest is checked.
// @ts-expect-error
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
// @ts-expect-error
import { tmpdir } from "node:os";
// @ts-expect-error
import { fileState, handleStop } from "./reply-language-judge.ts";

writeFileSync(
  1,
  handleStop(
    readFileSync(0, "utf8"),
    fileState(`${tmpdir()}/claude-reply-language`, {
      read: (path) => readFileSync(path, "utf8"),
      write: (path, text) => writeFileSync(path, text),
      mkdir: (path) => mkdirSync(path, { recursive: true }),
    }),
  ),
);

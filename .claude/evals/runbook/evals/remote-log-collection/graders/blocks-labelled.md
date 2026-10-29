---
type: llm
---

Look at every block of commands the user is to run. A block that shows file contents to type into an editor is not a command block.

A label counts when it sits on the block itself or on the step heading or step text the block belongs to, such as "On app-3:" at the start of a numbered step; it covers every command block in that step.

PASS if each command block is covered by a label that says whether it runs on the laptop or on `app-3`.

FAIL if any command block is covered by no such label, so the executor has to infer where it runs — for example from an `ssh app-3` prefix inside the command, or from a sentence at the top of the reply.

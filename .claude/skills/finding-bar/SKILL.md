---
name: finding-bar
description: "The bar a review finding must clear to hold, and the proposals that carry a bar of their own instead. Stated once for both sides of a review, so what a reviewer raises and what a fixing side accepts are weighed against the same words. Read it when raising a finding, and when judging one."
---

# The Finding Bar

A finding holds by naming the wrong action or outcome that follows from leaving it unfixed.
One that names none argues a preference rather than a defect.
Its size does not enter: a named outcome is a finding however small.

What has to be nameable is the input or condition and the wrong outcome it produces.
Where what is under review is prose instead, it is who reads the thing, what they do wrong under it, and what that costs them.

Some shapes of finding reach that only through a form of their own:

- For a behavior the tests do not pin, it is the wrong behavior that would go undetected, where the behavior is one the work under review must protect: a test pinning anything else is what the next correct change undoes.
- For a maintainability finding, it is the future cost — what a later change is made to do twice, or to undo.
- For a statement that is false, it is what whoever writes or copies from it next does: context around it can keep a reader from acting on it, which does not make the statement true.
- For a value the change takes in rather than produces, it is what whoever supplies that value can make the code branch on, or can make a reader of that value do, such as an agent that executes prose the value reaches.
- For a report, exit code, preview or alert, it is the look its reader does not take: a signal can be wrong by staying quiet, and one that is right per item can still be wrong in aggregate.

Some proposals carry a bar of their own in place of that one.

- **A removal.** Name what the removed thing was there to prevent, and what prevents that now.
- **A violation of a written standard** — any written rule the work under review is subject to, the usual carriers being a rule or instruction file that loads for its paths and a skill that governs the kind of work. Name the standard's file and what it states, and its harm is settled by the standard rather than argued.

Each side of a review does something different with a finding that clears this, and each states that for itself.

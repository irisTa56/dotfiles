---
name: address-finding
description: "Decide whether and how to act on a review finding or comment, then apply a fix that resolves the root cause instead of piling on a patch. The shared judgment-and-fix core used by review-loop and address-review-comment; also applies whenever you are acting on code-review feedback."
---

# Address Finding

Given a review finding or comment, decide the response and — when a fix is warranted — apply one that keeps the deliverable coherent as a whole rather than accreting local patches.

## 1. Evaluate validity

Take the finding seriously, but do not accept it blindly.

- **Technically correct?** Does it fix a real bug, correctness gap, or genuine concern?
- **Improves quality?** Readability, maintainability, or performance.
- **Trade-offs?** Consider what the finding may have missed.
- **Proportionate?** Judge whether the proposed approach is appropriately scoped; a simpler, more targeted fix may resolve the same concern. This weighs the *how*, not the *whether* — the underlying problem should still be addressed.
- **A more fundamental fix?** Reframe holistically: is there a root-cause fix that removes repeated manual work or prevents the whole class of issue (automation/abstraction over ad-hoc edits)?
- **Broken-window risk?** Even if minor now, fix it when leaving it invites further degradation or costly rework.
- **Grounded in actual behavior?** Judge by what the code does, not by what a spec or plan says. If the spec or plan itself looks flawed, surface it to the user rather than silently conforming.

Land on one verdict:

| Verdict | Action |
|---------|--------|
| Valid | Apply the fix as suggested, or a better variant addressing the same concern |
| Valid but not fundamental | Apply a more structural fix; explain why it is safer / lower future cost |
| Partially valid | Propose a balanced fix and explain the reasoning |
| Not valid | Prepare a respectful explanation of why |

## 2. Apply the fix

### The purpose bound

Establish the purpose of the change in hand before weighing any fix against it.
Take it from the first of these that states one:

- the user;
- the calling workflow;
- the diff, read as a whole.

State the purpose and the source you took it from, so the user sees the bound they are judging against.

The part of a fix that reaches past that purpose, whatever its shape, stops for the user before it is applied.
Where this skill prescribes surfacing instead of applying — a prescribing claim the fix contradicts, or a spec or plan judged flawed — the stop does not apply: nothing lands, so there is nothing to stop.
One thing lands outside the bound without stopping: a describing claim the fix itself falsified, for the reason the axis below gives.

The part of the fix inside the bound is kept whichever option is chosen; what the user decides is only the part that reaches past it.
Put that part to them with the real options, rather than a default to deferral:

- extend the change to cover it;
- carve it out, to be done separately;
- leave it undone.

Recommend one and say why, then apply what the user decides.

### Fix discipline

- **Root cause, minimal scope.** Fix the underlying cause; do not bundle unrelated refactors.
- **No silent reversal.** Check the fix against the decisions already taken in this piece of work, and not only the most recent. When it undoes one of them, say so and argue why the reversal is right rather than letting it land as a fresh fix.
  - By default the piece of work is the change in hand together with any fixes made on top of it.
  - A calling workflow may name a wider scope.
- **Consistency sweep.** Sweep the axes below, and leave no correction half-applied.
  - **Sibling sites.** Fix the same defect wherever else it appears — other call sites, sibling functions, similar files. When the spread reaches past the purpose bound, fix what falls inside it and put the rest to the user under the stop above.
  - **Falsified claims.** Update a claim that *describes* what the code does, wherever it lives — docstrings, comments, documents.
    - This is breakage the fix caused, not a defect it found, so the purpose bound does not cap it.
    - A claim that *prescribes* the behaviour, as in a spec, a plan, or an ADR, is not rewritten. The fix contradicts it, so surface that to the user rather than silently making it match.
    - A claim that reads both ways is treated as prescribing, and surfaced rather than rewritten.
  - **Implied counterpart.** When the fix establishes a contract, guard, or invariant that only one side of a pair now honours, decide explicitly whether the other side needs it, and tell the user why when it does not. When that other side falls outside the purpose bound, the stop applies before the counterpart edit lands.

## 3. Reflective checkpoint — suspect a band-aid

Before settling on the fix, look at what it does to the code:

- If it *adds* another branch, guard, flag, or special case onto an existing pile, treat that as a **signal — not a verdict** — that you may be treating a symptom. Step back and look for the root cause, or a consolidation that dissolves the pile.
- Every addition is a cost, not a neutral act — accept it reluctantly, because the goal cannot be met otherwise, never because the diff still looks acceptable.
- But do not let that reluctance harden into leanness as a goal in itself: when a valid fix genuinely requires the addition, withholding it to keep the diff small is the failure, not the fix.
- When stepping back enlarges the fix, weigh the enlarged fix against the purpose bound again before applying it.

## 4. Handling several findings at once

When multiple findings are in play, view them together before fixing.
Prefer one structural fix that dissolves a cluster of related findings over N independent local patches.
A structural fix can reach further than any of the patches it replaces, so weigh it against the purpose bound before applying it.

## Anti-patterns

Do not:

- dismiss a finding because "the existing code already does it this way";
- add a comment acknowledging the problem while leaving the code unchanged.

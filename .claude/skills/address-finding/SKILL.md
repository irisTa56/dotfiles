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

- **Root cause, minimal scope.** Fix the underlying cause; do not bundle unrelated refactors.
- **Consistency sweep.** Check related sites (other call sites, sibling functions, similar files) for the same issue and fix them together, so no partial correction remains. If the sweep exposes a systemic spread, fix what is practical here and flag the remainder to the user for a follow-up.

## 3. Reflective checkpoint — suspect a band-aid

Before settling on the fix, look at what it does to the code:

- If it *adds* another branch, guard, flag, or special case onto an existing pile, treat that as a **signal — not a verdict** — that you may be treating a symptom. Step back and look for the root cause, or a consolidation that dissolves the pile.
- Every addition is a cost, not a neutral act — accept it reluctantly, because the goal cannot be met otherwise, never because the diff still looks acceptable.
- But do not let that reluctance harden into leanness as a goal in itself: when a valid fix genuinely requires the addition, withholding it to keep the diff small is the failure, not the fix.

## 4. Handling several findings at once

When multiple findings are in play, view them together before fixing.
Prefer one structural fix that dissolves a cluster of related findings over N independent local patches.

## Anti-patterns

Do not:

- dismiss a finding because "the existing code already does it this way";
- add a comment acknowledging the problem while leaving the code unchanged.

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
- **The same mechanism again?** When findings keep landing on one mechanism, round after round, the mechanism is what is wrong and not its wording. Ask what it should do before rewording it again.
- **Broken-window risk?** Even if minor now, fix it when leaving it invites further degradation or costly rework.
- **Grounded in actual behavior?** Judge by what the code does, not by what a spec or plan says. If the spec or plan itself looks flawed, surface it to the user rather than silently conforming.
- **Does it show harm?** A finding that names no wrong action or outcome argues a preference, not a defect. Answer it with the reason what is there was chosen, and leave it standing — but name a reason you can point to, and when there is none, the choice is undefended, so judge whether the other criteria establish the harm the finding left unargued.
- **A removal proposal?** Judge it by §3's removal question.
- **Would the reading actually be taken?** Accept an ambiguity finding when the wrong reading is the literal or the cheaper one, or when readers demonstrably took it; reject one that had to be constructed.
  - Where the wrong reading wins by being cheaper, ask also whether the right procedure simply costs too much — the fix may be to cheapen it, not to reword.
- **What does the rewrite cost?** A rewrite is itself unreviewed, and is where the next defect comes from. Spend that only on a finding that shows harm.

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

The part of a fix that reaches past that purpose, whatever its shape, lands only on the user's answer.
Weigh the fix as it will finally be composed, once §3 and §4 have had their say, so the user is asked once.
Where this skill prescribes surfacing instead of applying — a prescribing claim the fix contradicts, or a spec or plan judged flawed — this does not add a second surfacing: nothing lands, so there is nothing to hold back.
One thing lands outside the bound on this skill's own authority: a describing claim the fix itself falsified, for the reason the sweep below gives.

The part of the fix inside the bound is kept whichever option is chosen, and so is anything this skill lands outside it on its own; what is put up for decision is only the part that reaches past the bound.
Put that part to them with the real options, rather than a default to deferral:

- extend the change to cover it;
- carve it out, to be done separately;
- leave it undone.

Recommend one and say why, then carry out what they choose.

What this skill owes is that surfacing.
Two things belong to the calling workflow instead, since it is the one that knows what stopping costs, and are for it to state:

- whether work waits for the answer;
- whether it has pre-committed an answer for a named class of excess — an answer that lands nothing, carve out or leave.

A pre-committed answer still surfaces, and still names the class it covers, so the user can overturn it.
Absent any statement, put it to the user and wait.

### Fix discipline

- **Meet the bar the fix will be judged against.** A calling workflow that reviews the result states what its reviewer weighs — `review-loop`'s "Perspectives for the Review" is one. Read it and hold the fix to it, rather than learning what it asks for from the next round's findings.
- **Root cause, minimal scope.** Fix the underlying cause; do not bundle unrelated refactors.
- **No silent reversal.** Check the fix against the decisions already taken in this piece of work, and not only the most recent. When it undoes one of them, say so and argue why the reversal is right; when it is not right, take a fix that leaves the earlier decision standing.
  - By default the piece of work is the change in hand together with any fixes made on top of it.
  - A calling workflow may name a wider scope.
- **Consistency sweep.** Sweep the axes below; each says how far its correction reaches.
  - **Sibling sites.** Fix the same defect wherever else it appears — other call sites, sibling functions, similar files.
    - When the spread reaches past the purpose bound, fix what falls inside it and surface the rest as the bound above prescribes.
  - **Falsified claims.** Update a claim that *describes* what the code does, wherever it lives — docstrings, comments, documents.
    - This is breakage the fix caused, not a defect it found, so the purpose bound does not cap it.
    - A claim that *prescribes* the behaviour, as in a spec, a plan, or an ADR, is not rewritten. The fix contradicts it, so surface that to the user rather than silently making it match.
    - A claim that reads both ways is treated as prescribing, and surfaced rather than rewritten.
  - **Implied counterpart.** When the fix establishes a contract, guard, or invariant that only one side of a pair now honours, decide explicitly whether the other side needs it.
    - Tell the user why when it does not.
    - When that other side falls outside the purpose bound, surface it as the bound above prescribes before the counterpart edit lands.

## 3. Reflective checkpoint — what the fix costs

Before settling on the fix, look at what it does to the code.

Every addition is a cost, counted in what a developer has to hold in mind to work here — not in the size of the diff.

That a finding is valid does not settle what to do about it.
Three questions answer that, and all three are weighed rather than taking the first that comes back non-empty.

- What can be removed, so the problem cannot arise?
  - Cheapest on the metric above, so ask it first.
  - Ask of it what the removed thing was there to prevent, and what prevents that now.
  - Deleting what the finding was about without answering that is not a fix; a removal that answers it is often the best one there is.
- What can be reshaped, so the problem does not arise here?
  - A reshape is a rewrite, so §1's rewrite cost applies.
  - Sometimes an addition is the smaller unreviewed surface.
- What has to be added?
  - Where only this answers the finding, it is the fix and not a last resort to be talked out of.

A removal you do not fully understand is riskier than an addition you do, and so is a fix bought with cleverness — avoiding an addition by being indirect, implicit, or surprising costs the developer more than the addition would have.
Adding another branch, guard, flag, or special case onto an existing pile is a signal you may be treating a symptom; step back for the root cause, or for a consolidation that dissolves the pile.

## 4. Handling several findings at once

When multiple findings are in play, view them together before fixing.
Prefer one structural fix that dissolves a cluster of related findings over N independent local patches.

## Anti-patterns

Do not:

- dismiss a finding because "the existing code already does it this way";
- add a comment acknowledging the problem while leaving the code unchanged.

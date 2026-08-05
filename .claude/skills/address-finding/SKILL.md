---
name: address-finding
description: "Decide whether and how to act on a review finding or comment, then apply a fix that resolves the root cause instead of piling on a patch. The shared judgment-and-fix core the review workflows apply; also applies whenever you are acting on code-review feedback."
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
  - One finding can carry that signal on its own: where what it calls a contradiction sits between two parts of the change itself, ask whether a decision is missing before asking which side is wrong, since two parts answering one question differently is what an unsettled specification looks like and picking a side buries it.
- **Broken-window risk?** Even if minor now, fix it when leaving it invites further degradation or costly rework.
- **Grounded in actual behavior?** Judge by what the deliverable does — the code when run, the prose when read — not by what a spec or plan says. If the spec or plan itself looks flawed, surface it to the user rather than silently conforming.
- **Does it show harm?** A finding that names no wrong action or outcome argues a preference, not a defect. Answer it with the reason what is there was chosen, and leave it standing — but name a reason you can point to, and when there is none, the choice is undefended, so judge whether the other criteria establish the harm the finding left unargued.
  - What has to be nameable, by you where the finding left it out, is the input or condition and the wrong outcome it produces; where what is under review is prose instead, it is who reads the thing, what they do wrong under it, and what that costs them.
  - A cheap fix is not a free one: cheapness answers what it costs to write and says nothing about whether anything went wrong without it.
- **A standard already in force?** A violation of a written standard that loads into your own context when you work on the file, such as a rule file or an instruction file, has its harm settled by the standard and owes no separate harm argument.
  - Verify the standard says what the finding claims and that its scope reaches the file; one that fails that check argues a preference.
- **A removal proposal?** Judge it by §3's removal question.
- **Does the context resolve it?** The statement a reader meets is the one the context around it carries, so where the finding is about prose rather than code, reject what only stands when it is read apart from that — an inconsistency, or a harmful parse the next clause or the step before rules out — by naming what resolves it.
  - Prose is read in context, so that naming answers the finding; picking sentences apart one by one produces conflicts without end.
  - A misreading a reader or a call site demonstrably made is not answered this way, since the demonstration is what naming the context would have to overturn.
- **What does the rewrite cost?** A rewrite is itself unreviewed, and is where the next defect comes from. Spend that only on a finding that shows harm.

Land on one verdict: the finding holds, it does not, or only part of it does.
The part that holds owes a fix, whose shape §3 settles rather than this section.
The part that does not owes a respectful explanation of why, and nothing else.

## 2. The purpose bound

Establish the purpose of the change in hand before weighing any fix against it.
Take it from the first of these that states one:

- the user;
- the calling workflow;
- the diff, read as a whole.

State the purpose and the source you took it from, so the user sees the bound they are judging against.

The bound is the outcome, not the file set.
A fix in a file the change has not touched is inside it where the purpose is not served without that fix, and an edit inside a file the change already touches is past it where it serves some other outcome.

The part of a fix that reaches past that purpose, whatever its shape, lands only on the user's answer.
Weigh the fix as the sections below will finally compose it, the consistency sweep included, so the user is asked once.
Where this skill prescribes surfacing instead of applying — a prescribing claim the fix contradicts, or a spec or plan judged flawed — this does not add a second surfacing: nothing lands, so there is nothing to hold back.
One thing lands outside the bound on this skill's own authority: a describing claim the fix falsified, at whatever remove down the chain the sweep below follows, for the reason that sweep gives.

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

## 3. Choose the fix

Every addition is a cost, counted in what a developer has to hold in mind to work here — not in the size of the diff.

That a finding holds does not settle what to do about it.
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

Where several findings are in play, ask the three questions of them together: one structural fix that dissolves a cluster beats N independent local patches.

## 4. Apply the fix

### Fix discipline

- **Meet the bar the fix will be judged against.** A calling workflow that reviews the result states what its reviewer weighs. Read that statement and hold the fix to it, rather than learning what it asks for from the next review's findings.
- **No silent reversal.** Check the fix against the decisions already taken in this piece of work, and not only the most recent. When it undoes one of them, say so and argue why the reversal is right; when it is not right, take a fix that leaves the earlier decision standing.
  - By default the piece of work is the change in hand together with any fixes made on top of it.
  - A calling workflow may name a wider scope.
- **Establish the fix, rather than only arguing it.** §1 grounds the finding in actual behavior, and the fix answering it owes the same before it lands.
  - **What it takes as true.** Check what the fix asserts or assumes about a command, an API, or a mechanism, since a claim written from memory is unreviewed content that reads as established.
    - Docs state what a thing is specified to do, which is not always what it does, so a claim about behavior is settled by running it.
    - Run it where a wrong result costs nothing.
  - **What it does where it lands.** Read the passage or the path whole with the fix in place, as its reader meets it — a fix argued against one finding can be right about that finding and wrong about what it now governs, or about what its removal leaves uncovered.
- **Consistency sweep.** Sweep the axes below; each says how far its correction reaches.
  - **Sibling sites.** Fix the same defect wherever else it appears — other call sites, sibling functions, similar files.
    - When the spread reaches past the purpose bound, fix what falls inside it and surface the rest as the bound above prescribes.
  - **Falsified claims.** Update a claim that *describes* what the code does, wherever it lives — docstrings, comments, documents.
    - This is breakage the fix caused, not a defect it found, so the purpose bound does not cap it.
    - Run the axis again over the corrections themselves, until a pass finds nothing: a corrected claim falsifies the claims that described *it*, and where cross-references are dense that chain runs several sites deep.
    - Where the same class of claim keeps needing this, the reference structure is the defect rather than the staleness it produces: ask §1's same-mechanism question of the references themselves, and weigh §3's removal for them, since a fix that adds a reference makes the next chain longer.
      - Restructuring is not a falsified claim, so it is weighed under §3 like any other fix shape, and where it reaches past the purpose bound it is surfaced under §2 with the carve-out recommended, since the corrections already made stand without it.
    - A claim that *prescribes* the behaviour, as in a spec, a plan, or an ADR, is not rewritten. The fix contradicts it, so surface that to the user rather than silently making it match.
    - A claim that reads both ways is treated as prescribing, and surfaced rather than rewritten.
  - **Implied counterpart.** When the fix establishes a contract, guard, or invariant that only one side of a pair now honours, decide explicitly whether the other side needs it.
    - Tell the user why when it does not.
    - When that other side falls outside the purpose bound, surface it as the bound above prescribes before the counterpart edit lands.

## Anti-patterns

Do not:

- dismiss a finding because "the existing code already does it this way";
- add a comment acknowledging the problem while leaving the code unchanged.

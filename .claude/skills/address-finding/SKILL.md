---
name: address-finding
description: "Decide whether and how to act on a review finding or comment, then apply a fix that resolves the root cause instead of piling on a patch. The shared judgment-and-fix core the review workflows apply; also applies whenever you are acting on code-review feedback."
---

# Address Finding

Given a review finding or comment, decide the response and — when a fix is warranted — apply one that keeps the deliverable coherent as a whole rather than accreting local patches.

## 1. Evaluate validity

Take the finding seriously, but do not accept it blindly.

- **Improves quality?** Readability, maintainability, or performance.
- **Trade-offs?** Consider what the finding may have missed.
- **Proportionate?** Judge whether the proposed approach is appropriately scoped; a simpler, more targeted fix may resolve the same concern. This weighs the *how*, not the *whether* — the underlying problem should still be addressed.
- **A more fundamental fix?** Reframe holistically: is there a root-cause fix that removes repeated manual work or prevents the whole class of issue (automation/abstraction over ad-hoc edits)?
- **The same mechanism again?** When findings keep landing on one mechanism, round after round, the mechanism is what is wrong and not its wording. Ask what it should do before rewording it again.
  - One finding can carry that signal on its own: where what it calls a contradiction sits between two parts of the change itself, ask whether a decision is missing before asking which side is wrong, since two parts answering one question differently is what an unsettled specification looks like and picking a side buries it.
- **Broken-window risk?** Even if minor now, fix it when leaving it invites further degradation or costly rework.
- **Grounded in actual behavior?** Judge by what the deliverable does — the code when run, the prose when read — not by what a spec or plan says. If the spec or plan itself looks flawed, surface it to the user rather than silently conforming.
- **Does it show harm?** Weigh it against the bar the `finding-bar` skill states, naming for yourself what the finding left out.
  - One that does not clear it argues a preference rather than a defect: answer it with the reason what is there was chosen, and leave it standing.
  - Name a reason you can point to; where there is none the choice is undefended, so judge whether the other criteria establish the harm the finding left unargued.
  - A cheap fix is not a free one: cheapness answers what it costs to write and says nothing about whether anything went wrong without it.
- **A standard already in force?** A violation of one clears the bar on its own terms, per `finding-bar`.
  - Verify the standard says what the finding claims and that its scope reaches the file; one that fails that check argues a preference.
- **A removal proposal?** Judge it by §4's removal question.
- **Does the context resolve it?** The statement a reader meets is the one the context around it carries, so where the finding is about prose rather than code, reject what only stands when it is read apart from that — an inconsistency, or a harmful parse the next clause or the step before rules out — by naming what resolves it.
  - Prose is read in context, so that naming answers the finding; picking sentences apart one by one produces conflicts without end.
  - A misreading a reader or a call site demonstrably made is not answered this way, since the demonstration is what naming the context would have to overturn.
- **What does the rewrite cost?** A rewrite is itself unreviewed, and is where the next defect comes from. Spend that only on a finding that shows harm.

Land on one verdict: the finding holds, it does not, or only part of it does.
The part that holds owes a fix, whose shape §4 settles rather than this section.
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

## 3. Putting the excess to the user

The part of a fix that reaches past the bound, whatever its shape, lands only on an answer — the user's, or one pre-committed.
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
- whether it has pre-committed an answer for a named class of excess — one that lands nothing, or one whose undoing costs the user less than answering would have.

A pre-committed answer still surfaces, and still names the class it covers, so the user can overturn it.
Absent any statement, put it to the user and wait.

## 4. Choose the fix

Every addition is a cost, and you MUST count it in what a developer has to hold in mind to work here, NEVER in the size of the diff.

That a finding holds does not settle what to do about it.
Three questions answer that, and all three are weighed rather than taking the first that comes back non-empty.

- What can be removed, so the problem cannot arise?
  - Cheapest on the metric above, so ask it first.
  - Hold it to `finding-bar`'s removal bar.
  - Deleting what the finding was about without answering that is not a fix; a removal that answers it is often the best one there is.
- What can be reshaped, so the problem does not arise here?
  - A reshape is a rewrite, so §1's rewrite cost applies.
- What has to be added?
  - Where only this answers the finding, it is the fix and not a last resort to be talked out of.
    - What settles "only this" is whatever already covers the failure — a rule, a guard, a helper: where one did, it went unhonoured or unreached rather than missing, so a second of it fixes nothing and leaves one job in two places.

A removal you do not fully understand is riskier than an addition you do, and so is a fix bought with cleverness — avoiding an addition by being indirect, implicit, or surprising costs the developer more than the addition would have.
Adding another branch, guard, flag, or special case onto an existing pile is a signal you may be treating a symptom; you MUST step back for the root cause, or for a consolidation that dissolves the pile.

Where several findings are in play, you MUST ask the three questions of them together: one structural fix that dissolves a cluster beats N independent local patches.

## 5. Apply the fix

- **Meet the bar the fix will be judged against.** It is `finding-bar`'s, which the reviewer of a calling workflow weighs too, together with whatever that workflow states alongside it.
  - That is a narrowing of the bar, an addition to it, or a constraint the fix must keep.
  - Read that statement and hold the fix to the result, rather than learning what it asks for from the next review's findings.
- **No silent reversal.** Check the fix against the decisions already taken in this piece of work, and not only the most recent. When it undoes one of them, say so and argue why the reversal is right; when it is not right, take a fix that leaves the earlier decision standing.
  - By default the piece of work is the change in hand together with any fixes made on top of it.
  - A calling workflow may name a wider scope.
- **Establish the fix, rather than only arguing it.** §1 grounds the finding in actual behavior, and the fix answering it owes the same before it lands.
  - Check what the fix asserts or assumes about a command, an API, or a mechanism, since a claim written from memory is unreviewed content that reads as established.
  - Docs state what a thing is specified to do, which is not always what it does, so a claim about behavior is settled by running it.
  - Run it where a wrong result costs nothing.
- **Read it whole where it lands.** You MUST read the passage or the path whole with the fix in place, as its reader meets it — a fix argued against one finding can be right about that finding and wrong about what it now governs, or about what its removal leaves uncovered.
  - Where the fix changes a predicate, a gate, or a probe, enumerate the inputs it now answers for and say what each one gets, rather than reading for them: the one case that raised the finding is not what the fix governs.
- **Consistency sweep.** Sweep the axes below.
  - **What holds for every axis.**
    - The purpose bound caps every correction the sweep makes but one — a describing claim the fix falsified, for the reason "Falsified claims" gives — so fix what falls inside it and surface the rest as §3 prescribes.
    - Run them again over their own corrections, until a pass finds nothing: a correction falsifies the claims that described it and can break what cites it, and where cross-references are dense that chain runs several sites deep.
    - A claim that *prescribes* the behaviour, as in a spec, a plan, or an ADR, is not rewritten where the fix contradicts it, whichever axis reached it.
      - Surface that contradiction to the user rather than silently making the claim match.
    - A claim that reads both ways is treated as prescribing.
    - Where the same sites, or the same class of them, keep needing correction, the reference structure is the defect rather than what it produces: ask §1's same-mechanism question of the references themselves, and weigh §4's removal for them, since a fix that adds a reference makes the next chain longer.
      - Restructuring is not one of these corrections, so it is weighed under §4 like any other fix shape, and where it reaches past the purpose bound it is surfaced under §3.
  - **The axes.**
    - **Dependent sites.** Follow what calls or cites the fixed site, and repair what no longer lands where it did.
      - What breaks is whatever relied on the scope the site had, so a removal is the limiting case rather than an exception.
      - Ask of each site you reach whether it still lands rather than whether it still resolves.
    - **Sibling sites.** Fix the same defect wherever else it appears — other call sites, sibling functions, similar files.
    - **Falsified claims.** Update a claim that *describes* what the code does, wherever it lives — docstrings, comments, documents.
      - Staleness the fix caused is not a defect it found.
    - **Implied counterpart.** When the fix establishes a contract, guard, or invariant that only one side of a pair now honours, decide explicitly whether the other side needs it.
      - Tell the user why when it does not.

## Anti-patterns

Do not:

- dismiss a finding because "the existing code already does it this way";
- add a comment acknowledging the problem while leaving the code unchanged.

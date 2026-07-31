---
name: review-loop
description: "Orchestrate an iterate-until-clean review of code you just changed: delegate the review to a subagent, judge each finding, then loop fix and re-review until a round applies no fix, reporting at the end what was left undone and why. Invoke when the user asks to pass the current changes through review, or when a workflow step calls for a review pass. The subagent does the actual review via the code-review-expert skill and makes no edits."
---

# Review Loop

## The record

The loop keeps its state in a file, so it survives a compaction and a session that ends mid-loop; re-read it whenever your own context no longer holds what it says.
It lives at `$(git rev-parse --path-format=absolute --git-common-dir)/review-loop/$(git rev-parse --abbrev-ref HEAD).md` (create the leading directories) — under the common git dir, which linked worktrees share and `git worktree remove` leaves alone, so it never reaches the reviewer through `git status`.
On a detached HEAD the branch resolves to `HEAD`, which is as much scoping as there is; say so and carry on.
Alongside it, a fresh loop's step 1 saves the change as it stands — the `git diff HEAD` output with untracked files included, by a means that leaves the index untouched, as `<branch>.base.diff` beside the record — so what the rounds altered stays derivable by comparison; a resumed loop keeps the baseline it finds.

It has two sections, and the headings are bookkeeping that stays in the file:

- `## Background (sent to the reviewer)` — what is true of the change, covering what only this side knows and the diff does not show:
  - the pinned purpose;
  - what was decided along the way, and why, marked as the user's, the loop's, or the invoking workflow's;
  - the use the change is built for;
  - the constraints the deliverable must keep;
  - the reasoning behind the design choices the change embodies.
- `## Verdicts (never sent)` — entries grouped by round, each entry holding:
  - what was raised;
  - what was decided;
  - the reason, written to be weighed rather than taken on trust;
  - whether the loop, the user, or the invoking workflow decided it.

Each round's group also notes the fixes it landed and what it added to the background; against the saved baseline, that is enough for a resumed loop to read convergence and the cut confinement.

The line between the sections: "the caller validates this already, so checking it here is deliberately out of scope" is background; "a reviewer raised that and we rejected it" is a verdict.
Anything worth carrying can be put the first way, and handing a reviewer the second is how it stops looking.

## Workflow

1. **Establish the purpose.** State what this change is for before the first round, and hold every later round to that statement — re-inferring it from a diff the rounds grew turns the bound into a ratchet.
   - Find this work's record before anything else: this branch's path first, then — since a branch switch strands a record under the old name — the rest of `review-loop/`, at every depth, for an unmarked record whose content matches.
     - An unmarked record of this work at this branch's path is an interrupted loop: resume it, adding to its background rather than composing one over it.
     - A record whose last line is `## Closed` is a finished loop's: set it aside under a name that says which loop it was.
     - Anything else — a match under another branch's name, a record you cannot place — goes to the user before you touch it.
       - One they disown is closed and set aside likewise.
       - One that is this work moved is renamed here, its baseline with it, keeping any occupant beside it.
     - Nothing found means a fresh loop: create the record.
   - Take the purpose from the first of these that states one, and say which source it came from:
     - the user;
     - a resumed record;
     - the workflow that invoked this loop;
     - the diff, read as a whole.
   - State the purpose as the outcome the change is for; where the background also names the files the change touches, that describes the change and does not narrow the bound.
   - Do not wait for a reply: state it, with the rest of the background you composed, and carry on — a correction is available and costs nothing to skip.
   - The pinned purpose as the background currently holds it serves as `address-finding`'s purpose statement for every round; restate it only when putting an excess to the user, who is judging against it.
     - The constraints the background states go to it alongside, as part of the bar its fix discipline holds the fix to, so a fix that breaks one is caught before it lands rather than in the next round's findings.
   - Fill the background section before the first round.
     - Enter inferences as inferences: a guess dressed as a user decision misleads every reviewer and would be shielded from re-judging.
   - Work that has landed is in-bound for later rounds, and licenses no further excursion past the purpose.
2. **Review in a subagent.** Spawn a general-purpose subagent (the `Agent` tool) and, in its prompt, instruct it to review the current changes by running the `code-review-expert` skill with the section below.
   - `code-review-expert` is a Skill, not an agent type — do NOT pass it as `subagent_type`, since that call fails.
   - Choose the reviewer's model rather than letting it inherit, since inheritance takes the tier of whatever spawned it.
     - A chain that already delegated to a cheaper model silently checks its own work at that tier, and a run on a premium tier silently pays that premium a second time.
     - Spawn the reviewer on the `opus` tier, unless the user or the invoking workflow named a higher one for this reviewer.
     - Where the running model family offers no such tier, name the tiers it does offer and ask, rather than picking one.
   - The subagent returns findings only and makes no edits; it skips `code-review-expert`'s closing step that asks which findings to fix, since step 4 decides that.
   - Bar it from reading under the common git dir.
   - Beyond those run constraints, the prompt carries the changes, the section below, and the background section's contents copied whole; never quote the verdict section.
   - Compose no review instructions of your own on top, beyond what an endgame in force tightens.
3. **Report findings** to the user as the subagent returned them.
4. **Judge and fix with `address-finding`.** Apply the `address-finding` skill (invoke it via the Skill tool) to judge each finding's validity and fix the valid ones, stating which you accept or reject and why.
   - A finding the floor covers is neither judged nor fixed for its own sake: report it at the close.
     - The floor covers the lowest rank of the reviewer's own scale, what it marked optional or left in a bucket it marks non-blocking, and a finding against the background on the same terms unless that finding shows a statement there false.
     - That falsity exception reaches past the background: a finding naming a statement anywhere that the change itself made false or misleading is judged and fixed at whatever rank it carries.
       - The change caused that one, so leaving it ships an artifact that says something untrue, which no rank makes minor.
       - Check what the statement said before the change; one already wrong is a defect the change found rather than one it caused, and stays on the floor.
     - A removal such a finding proposes is taken in any round that is landing a fix the floor does not cover, since that round is re-reviewed anyway and what it prunes is surface the next round would read.
     - A violation of a written standard in force over what is under review is taken on a removal's terms, at whatever rank it carries.
       - The floor keeps the loop from spending judgment on preference, and a standard the user already settled costs none; its rank measures the reviewer's sense of impact rather than whether the standard holds.
       - Put it through `address-finding`'s standard criterion before it rides, and reject the finding where it fails there, or a preference dressed as a rule buys its way past the floor.
     - Anything else a floored finding proposes is taken only where a fix the floor does not cover already edits what it names.
     - Whichever way a fix rides in, it is checked against the purpose, the decisions already taken, and the sites it touches; the finding itself is not settled, so no demonstration is owed for it.
   - Enter every outcome in the verdict section:
     - a finding you accepted, with the fix it landed;
     - a finding you rejected, with why it does not hold;
     - a finding the floor left unfixed, with the rank it carried or none, so the close still reaches it;
     - an answer to a hold, with what the continue narrows the round to, so a resumed loop still holds both;
     - a probe's task and what it produced, whichever way it settled the finding, so a later round can tell what was tested;
     - a spec or plan judged flawed, and a prescribing claim the fix contradicts — `address-finding` is barred from fixing either;
     - a valid finding the loop cannot fix, with what became of it and who chose that;
     - a fix that chose between competing options — which option and why goes into the background too, or a later round proposes the reverse to a reviewer never told the choice existed;
     - a fix that departed from what the user asked, with the departure named;
     - a valid finding the user directed the loop to leave, and every part the purpose bound held back — each decided one goes into the background too, as what is true of the change;
     - an objection to a user decision, set aside unjudged, for the close report to pick up; raised again on the same grounds, it is answered from the record.
   - A rejection resting on something checkable — what a command returns, what the suite already covers, what a probe produced — puts that fact in the background as well, so the next reviewer reads it rather than deriving it again; the verdict itself stays where it is.
     - Each such fact carries what established it — the command and what it returned, the test that covers the behavior, the probe's task — or a later reviewer cannot tell whether its own concern is the one that was answered.
   - Trying the text on a reader is the loop's to run, not the reviewer's: probe where a finding turns on how a reader would take the text, the argument has not settled it, and the floor does not already cover it.
     - Put the text where its reader would meet it, and a task its scenario calls for, to a fresh subagent whose prompt bars it from changing anything or reading under the common git dir, and read the wrong act off what it produces.
     - Compose the task without the finding's framing or the reading it names — a reader handed the wrong reading takes it, and one asked about a sentence finds it.
     - A run that never engaged the task shows nothing and is replaced.
     - `address-finding`'s ambiguity criterion accepts an ambiguity finding in any round, and only on a misexecution demonstrated to do real harm — this probe's, a reader's, or a call site's; its argued routes, a literal or cheaper parse, no longer accept one.
       - Argument alone rejects only by showing the reading is not available in the text; that no reader would take an available one is a claim about readers, which a probe settles.
   - A valid finding against the background rather than the change is answered by correcting the background; never edit the change to make an argument come out right.
     - A reason the user gave is not yours to rewrite: put the refutation to them in the round's message, without waiting, and their answer updates the background in their own words.
       - Enter it in the verdict section meanwhile; a later round objecting to that reason again is answered from the record.
       - An objection to the pinned purpose is the exception: it waits, as below.
   - Before applying any edit, read the verdict groups: when consecutive rounds' edits have landed mainly on what earlier rounds added — in the change or the background — and this round's accepted findings land there again, the loop is refining its own additions: hold the round's edits, put continuing to the user in the round's message, and wait.
     - Expect this on rule-bearing prose, where a fix is often a new rule and each rule adds surface for the next round to review.
     - Continue means apply and go on; stop means close with the held findings reported as left unfixed — so no fix ever ships unreviewed.
     - The message says how many rounds the loop has run from its start, so a continue is weighed against what it has already taken.
     - Reading those groups, name the mechanism the rounds' findings have clustered on under `address-finding`'s same-mechanism signal, or say there is none.
     - Where one stands, put its own decision rather than continuing alone: the answers the rounds have moved between, what each costs, and which you recommend.
       - Their answer goes into the background as theirs, with the reason they gave, and a later round reopening the mechanism is answered from there.
       - One that settles the mechanism and says nothing of going on is a continue under what it settled, narrowed as the message recommended.
       - A settlement re-frames the round's held edits: re-weigh them against it before applying.
     - Recommend the endgame as the default answer: continue, with the round narrowed at both ends.
       - It applies nothing unless a valid finding fixes a defect the floor does not cover, and where one does it applies every such fix with every valid removal and every valid standard violation the round raised at whatever rank, and nothing besides.
       - Its reviewer is asked to raise, at full scope, only what such a round can apply, since every weaker finding is bought and discarded:
         - a defect whose outcome does real harm;
         - a removal under its own bar;
         - a violation of a standard in force under its own;
         - a statement the change itself made false or misleading.
     - Their stop voids the round's other answers; the close names what it voided.
       - A mechanism settlement is not among them: it settles the specification rather than the round, and reaches the background and the close report either way.
     - Their continue governs every round that follows, restarting the tally rather than retiring the question.
       - Consecutive rounds are counted afresh from the continue, and a round that only removed does not count.
       - What earlier rounds added keeps counting from the loop's start.
       - When the condition is met again, hold again.
   - Where the round needs the user, everything it needs them for goes to them together, in one message:
     - A mechanism decision the hold put is the user's alone: nothing below answers it in their place, and where they cannot answer it the loop goes no further under any of its answers — returning to the workflow where one invoked the loop, whatever that workflow prescribed about continuing, and stopping where none did.
       - A user who continues without settling it has left it, and it carries to the close as their unanswered question does.
     - Where the user cannot answer, do what the invoking workflow prescribed for what the message waits on; its answers count under each question's own bounds, as its own and never the user's.
       - A prescription is entered in the verdict section when first read, so a resumed loop still holds it.
     - Where no workflow invoked the loop and no mechanism stands unanswered, a waiting hold takes its recommended default, which counts as a continue under the hold's rules, and everything else carries to the close unanswered.
     - What no prescription covers returns to the workflow: enter the held state — the questions, what each answer would change, the would-be verdicts — in the verdict section, leave the record unclosed, and end the run with the questions as its result; the run that brings the answers resumes from there.
     - A fix reaching past the purpose bound puts one question — extend the change, carve it out, or leave it.
       - Carve it out yourself, without waiting, where the excess separates cleanly — what stays behind is coherent and correct without it.
       - Tell the user in the round's message, so they can extend the change instead; a carve-out they only learn about at the close is work that quietly never happens.
         - Their extend pulls the carved-out part back into the round's work.
       - Wait where it does not separate cleanly.
     - A valid finding the loop cannot fix is put as carve it out or leave it, and waits for the answer.
     - Wait on a reviewer's objection to the pinned purpose, the first time it is raised — only the user can re-pin it.
       - Where the user cannot answer, it travels as the round's other questions do — returned, or carried to the close — with the pin standing either way.
       - Their answer goes into the background, and a later round raising it again is answered from there.
       - A re-pin re-frames the round's other questions: re-weigh them against it before acting on their answers.
     - Do not wait on a flawed spec or a contradicted prescribing claim: the loop is barred from fixing either whatever the answer.
       - Surface each once; a later round raising it again is answered from the record.
   - A finding a later round raises again is judged again on its merits, from the record's reasoning rather than its verdict.
     - A loop spawning fresh reviewers produces recurrence by design, so recurrence is not evidence against the verdict.
     - Distinct findings clustering on one mechanism are `address-finding`'s same-mechanism signal, not recurrence.
     - A decision the user gave in so many words — an instruction, or their answer to a question put to them — is not re-judged; one read from their context is an inference and stays review surface.
     - A carve-out the loop already made and told them of is answered from the record.
   - A decision's justification lives in the record, and the loop does not also write it into the change: a sentence a reader who never saw this review would not need is there to answer the last round, and belongs in the record.
   - For `address-finding`'s no-silent-reversal check, the piece of work is the change under review together with the fixes and record this loop has accumulated, not the current round alone.
5. **Loop.** Spawn a fresh review subagent and repeat until a round applies no fix (putting in the reviewer's own proposal unaltered is a fix); a round can return findings and still be the last, so long as it changed nothing.
   - Review coverage of the loop's own bookkeeping — the record, the baseline, the mechanics of holding and closing — is deliberately subordinated to termination, with the close report as the compensating control.
     - A finding whose whole content is that this bookkeeping went unreviewed is answered from this decision; a defect in the bookkeeping is judged like any other.
   - What gates the close is what reviewers see: a round that edited the change (cuts included) or added to or corrected the background cannot be the last, since no reviewer has read the result.
     - Verdict writes, background cuts or condensations, and the facts a rejection leaves — each carrying what established it, so a reviewer checks rather than trusts it — never extend the loop; a condensation's fidelity is bookkeeping coverage, per the decision above.
       - Correcting the background likewise, where the correction leaves what a later reviewer would look into unchanged; one that moves it either way extends the loop, since no reviewer has read the change under it.
   - Close every round by looking at the change whole before deciding whether to spawn, on the last round as much as any other — though a stop-close reports rather than cuts:
     - whether the fixes so far read as one coherent thing rather than a stack of patches;
     - whether the rounds have piled up more than a developer should have to hold in mind — cut what they piled up before it becomes what the next round reviews;
     - the background likewise, since it is sent to every reviewer and grows every round.
   - A cut this look makes is confined to what the rounds themselves put there, under the removal bar the section below sets; a decision entry step 4 requires may be condensed, never dropped.
   - A cut to the change, or a consolidation, goes through `address-finding` like any finding, purpose bound included.
   - When the loop closes, report what the user has to read:
     - what was left unfixed, and why, with what the floor covered reported together rather than one by one, so the rest is not buried in it;
       - a standard violation among what the floor covered is named on its own, with the standard it cites, since a lump reports the user's own settled rule as a preference;
     - any decision that departed materially from what they asked for, or from a plan they approved;
     - a prescribing claim the fix contradicts;
     - what the last round raised and you rejected, since no later pass had a chance to raise it again;
     - every objection to a user decision the rounds set aside, since the decision is theirs to re-take;
     - questions the rounds put to them that are still unanswered, with what each answer would change;
     - a question answered by anyone but them — who answered, with what, and whether the answer stood;
     - carve-outs, flawed specs, a mechanism they settled, and the questions above, which outlive the loop — the record holds them but is no lasting home, so say so and let the user move them somewhere that is.
   - Close the record by appending `## Closed` as its last line rather than deleting it, which would take the carve-outs, flawed specs, and unanswered questions with it.

## Perspectives for the Review

Weigh these on top of the `code-review-expert` defaults.

The background that comes with the change is under review with it — whether the change serves it, and whether it is sound — and is given to be weighed, not obeyed.
The purpose it states bounds the fix, not the finding: raise a defect wherever the change has one.
Give the parts it says nothing about their full share of your attention: a well-argued background draws the eye, and a defect hides in the part nobody wrote about.

Raise a finding only by naming the wrong action or outcome that follows from leaving it unfixed, whatever it is raised under; a removal proposal and a violation of a written standard are the exceptions, each meeting its own bar below.
One that names none argues a preference, and without that bar a careful reviewer generates findings without end.

- For an ambiguity, it is the wrong reading the literal or cheaper parse yields — or one a reader demonstrably took — and what they would do under it; that a second reading can be constructed is not itself the finding.
- For a behavior the tests do not pin, it is the wrong behavior that would go undetected.
- For a maintainability finding, it is the future cost — what a later change is made to do twice, or to undo.
- For a violation of a written standard in force over what is under review, its own bar is the citation: name the standard's file and what it states, so the fixing side checks it rather than takes it on trust.
- For a reason in the background, it is what the change keeps or omits on the strength of a ground that does not support it — "the surrounding code already does it this way" defends nothing; that the ground is weak is not by itself the finding, and the fix reaches the background only.

Ground a finding in something checkable wherever you can — what a command returns, what another file states, what the code does when run; one resting only on how a sentence could be read is the weakest kind.
Check what you can reach yourself, and raise a finding that turns on how a reader would take the text without trying it on a fresh reader — that trial belongs to whoever fixes.

Where what you are reviewing is prose that a person or an agent executes rather than code a machine runs, it is not underspecified by defect: it leaves to the reader's judgement what the reader can be trusted to judge.
Raise what would mislead, not what is merely open.
Reading each statement against the context around it is among what the reader is trusted to do, so a conflict that shows up only when one is read alone is not a finding.
Where it states a procedure, raise what its goal or its conditions get wrong before what its steps leave rough — the roughness of steps is inexhaustible, and what they leave open is what the reader is trusted to judge.

Where prose argues for a choice, what is under review is the choice, not the wording that defends it; prose that states a checkable fact stays review surface, wherever it sits.

Propose a removal where you see one in what is under review, naming what it was there to prevent and what prevents that now — that naming is a removal's bar — since a rule that has outlived its reason is easier to see for someone who did not write it.

1. Whether the change is organically integrated into the deliverable as a whole, rather than a surface-level feature implementation — coherent with the existing design and optimized in context, not bolted on.
2. Whether the automated tests are sound in quality and coverage, and whether they exercise externally observable behavior rather than internal implementation details.
   - Where the change alters externally observable behavior and you can see the suite, name as a finding any behavior it introduces or alters that the suite does not pin, whether established by reading or by running the code.
   - How it is tested is the implementation side's call.
3. Whether declarative tooling config (e.g. `pyproject.toml`, `ruff.toml`, `mise.toml`, `.gitignore`) is authored minimally — only deviations from default, with no pre-emptive defense.
   - Flag added lines that:
     - restate a tool's default value, rather than configuring only what a concrete, already-encountered problem requires;
     - pre-emptively ignore lint rules or add "just in case" suppressions for problems that have not occurred;
     - defensively pin or bound pre-1.0 dependency versions absent an observed break;
     - embed process/progress notes (e.g. "committed once Phase N lands") in shipped config.

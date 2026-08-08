---
name: review-loop
description: "Orchestrate an iterate-until-clean review of code you just changed: delegate the review to a subagent, judge each finding, then loop fix and re-review until a round applies no fix, reporting at the end what was left undone and why. Invoke when the user asks to pass the current changes through review, or when a workflow step calls for a review pass. The subagent does the actual review via the raise-findings skill and makes no edits."
---

# Review Loop

## The record

The loop keeps its state in a file, so it survives a compaction and a session that ends mid-loop; re-read it whenever your own context no longer holds what it says.
It lives at `$(git rev-parse --path-format=absolute --git-common-dir)/review-loop/$(git rev-parse --abbrev-ref HEAD).md` (create the leading directories) — under the common git dir, which linked worktrees share and `git worktree remove` leaves alone, so it never reaches the reviewer through `git status`.

It has two sections, and the headings are bookkeeping that stays in the file:

- `## Background (sent to the reviewer)` — what is true of the change, covering what only this side knows and the diff does not show:
  - the pinned purpose;
  - the use the change is built for;
  - the constraints the deliverable must keep, a decision's outcome among them where it constrains what the change may be;
  - what the rounds established about the change or what it runs against, each fact carrying what established it.
- `## Verdicts` — entries grouped by round, each entry holding:
  - what was raised;
  - what was decided;
  - the reason, written to be weighed rather than taken on trust;
  - whether the loop or the user decided it.

Each round's group also notes the fixes it landed and what it added to the background, which is enough for a resumed loop to read convergence and the cut confinement.

The line between the sections: the background holds what is true of the change, and the verdicts hold why anyone chose it.
"The caller validates this already, so checking it here is deliberately out of scope" is background; the argument that settled a finding, and the reasoning behind a design choice, are verdicts.
The verdict section is never sent whole or quoted; what a reviewer needs from it travels in the settled list step 2 derives.

## Workflow

1. **Establish the purpose.** State what this change is for before the first round, and hold every later round to that statement — re-inferring it from a diff the rounds grew turns the bound into a ratchet.
   - Find this work's record before anything else: this branch's path first, then — since a branch switch strands a record under the old name — the rest of `review-loop/`, at every depth, for an unmarked record whose content matches.
     - An unmarked record of this work at this branch's path is an interrupted loop: resume it, adding to its background rather than composing one over it.
     - A record whose last line is `## Closed` is a finished loop's: set it aside under a name that says which loop it was.
     - Anything else — a match under another branch's name, a record you cannot place — goes to the user before you touch it.
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
2. **Review in a subagent.** Spawn a general-purpose subagent (the `Agent` tool) and, in its prompt, instruct it to review the current changes by running the `raise-findings` skill.
   - `raise-findings` is a Skill, not an agent type — do NOT pass it as `subagent_type`, since that call fails.
   - Choose the reviewer's model rather than letting it inherit, since inheritance takes the tier of whatever spawned it.
     - A chain that already delegated to a cheaper model silently checks its own work at that tier, and a run on a premium tier silently pays that premium a second time.
     - Spawn the reviewer on the `opus` tier, unless the user or the invoking workflow named a higher one for this reviewer.
     - Where the running model family offers no such tier, name the tiers it does offer and ask, rather than picking one.
   - The subagent returns findings only and makes no edits.
   - Bar it from reading under the common git dir.
   - Beyond those run constraints the prompt carries four things, and nothing else out of the verdict section:
     - the changes, meaning the diff baseline to read them against and whatever no diff reaches, which `raise-findings`' own scoping section lists;
     - the background section's contents, copied whole;
     - the settled list this round derives from the verdict section, a line for each finding a round disposed of without landing a fix, giving what was raised and the ground it went on — an objection set aside unjudged among them, whose ground is the setting aside;
       - Judge what belongs on it rather than reading it off a category: a finding the floor covered was not settled unless the round judged it.
       - A finding joins it once a second round has raised it.
       - Write each line as what happened rather than as a verdict to honour: "raised in round 2, settled on X", not "X is not a problem".
       - Head the list with its one condition: raise one of these again only with evidence the list does not already answer.
     - what this loop adds to what `raise-findings` asks for: the purpose the background states bounds the fix and not the finding, so a defect is raised wherever the change has one.
   - Compose no review instructions of your own on top.
3. **Report findings** to the user as the subagent returned them.
   - Say with them which round this is counting from the loop's start, and the consecutive-round tally as the continue rule below counts it, every round and not only when the hold below fires, so a recurrence cannot pass unremarked.
4. **Judge and fix with `address-finding`.** Apply the `address-finding` skill (invoke it via the Skill tool) to judge each finding's validity and fix the valid ones, stating which you accept or reject and why.
   - The judging predicates are that skill's and this step points at them rather than restating one, so a correction lands where all its callers read it.
     - A narrowing the loop itself needs is not a restatement and stays here, as the settled list's re-raise condition below is, since step 4 is what those callers do not read.
   - What a finding is ranked is the reviewer's, and the verdict is yours: a rank orders the round and never answers whether the finding holds.
   - What a fix has to clear is `finding-bar`'s bar — or, where the finding is a removal, `finding-bar`'s removal bar, and where it is a standard violation, `address-finding`'s standard criterion — read off the deliverable and never off what the loop has spent; applying nothing is a round's correct outcome where nothing clears its bar.
   - A finding the floor covers is not fixed for its own sake.
     - The floor covers the lowest rank of the reviewer's own scale, and a finding against the background on the same terms unless it shows a statement there false.
     - That falsity exception is not confined to the background, and puts the finding outside the floor rather than past the bar above: a statement the change made false or misleading is fixed at whatever rank it carries where a reader acts on it, and rejected where none does.
       - Read what the statement said before the change: one already wrong is a defect the change found rather than one it caused, so it leaves the floor under the purpose bound rather than past it.
     - A finding proposing a removal, or naming a violation of a written standard at whatever rank, sits outside the floor: judge and fix it for its own sake, whatever else the round lands.
       - Put the standard violation through `address-finding`'s standard criterion first, and reject the finding where it fails there, or a preference dressed as a rule buys its way past the floor.
     - Anything else a floored finding proposes is applied only where a fix the floor does not cover already edits what it names.
     - What is applied that way is checked against the purpose, the decisions already taken, and the sites it touches.
   - Enter every outcome in the verdict section:
     - a finding you accepted, with the fix it landed, and what each axis of that fix's consistency sweep reached, or that the axis reached nothing;
     - a finding you rejected, with why it does not hold;
     - a finding at the floor's rank, the verdict it took, and what took it out of the floor or let a fix through where one landed;
     - an answer to a hold, with what the continue narrows the round to, so a resumed loop still holds both;
     - a probe's task and what it produced, whichever way it settled the finding, so a later round can tell what was tested;
     - a spec or plan judged flawed, and a prescribing claim the fix contradicts — `address-finding` is barred from fixing either;
     - a valid finding outside the floor that the loop cannot fix, with what became of it and who chose that;
     - a fix that chose between competing options — which option it took goes into the background as what is true of the change, and why it beat the others stays here;
     - a fix that departed from what the user asked, with the departure named;
     - a valid finding outside the floor that the user directed the loop to leave, and every part the purpose bound held back — what the change therefore does not cover goes into the background as what is true of it, and the reason stays here;
     - an objection to a user decision, set aside unjudged, for the close report to pick up.
   - A fact a round established rather than argued — `address-finding`'s work to settle a fix included — goes into the background, so the next reviewer reads it rather than deriving it again; whatever verdict it served stays in the verdict section.
     - Each such fact carries what established it — the command and what it returned, the test that covers the behavior, the probe's task — or a later reviewer cannot tell whether its own concern is the one that was answered.
     - A reviewer's argument is not one of those: establish it yourself before entering it — run the command, run the test, run the probe, or read the text — since what the background states is read as settled by every round after and none of them can see it was never checked.
   - Trying the text on a reader is the loop's to run, not the reviewer's: a finding claiming a reader acts wrongly under the text predicts a behaviour, and putting the text in front of one measures it.
     - Run it at your discretion, where argument has not settled the claim and the floor does not already cover it; no verdict waits on a probe, and a finding is accepted or rejected on argument where argument settles it.
     - Put the text where its reader would meet it, and a task its scenario calls for, to a fresh subagent whose prompt bars it from changing anything or reading under the common git dir, and read the wrong act off what it produces.
     - Compose the task without the finding's framing or the reading it names — a reader handed the wrong reading takes it, and one asked about a sentence finds it.
     - A run that never engaged the task shows nothing and is replaced.
     - What it produced is an established fact under the rule above, so a probe that refuted a finding is paid for once rather than once a round.
   - A valid finding outside the floor and against the background rather than the change is answered by correcting the background; never edit the change to make an argument come out right.
     - What the user stated in their own words — the purpose, a constraint, a question they settled — is not yours to rewrite: put the refutation to them in the round's message, without waiting, and their answer updates the background in their own words.
       - Enter it in the verdict section meanwhile.
       - An objection to the pinned purpose is the exception: it waits, as below.
   - Before applying any edit, read the verdict groups: when consecutive rounds' edits have landed mainly on what earlier rounds added — in the change or the background — and this round's accepted findings outside the floor land there again, the loop is refining its own additions: hold the round's edits, put the round's open question to the user in the round's message, and wait.
     - Expect this on rule-bearing prose, where a fix is often a new rule and each rule adds surface for the next round to review.
     - Continue means apply and go on; stop means close with the held findings reported as left unfixed — so no fix ever ships unreviewed.
     - The message carries the counts step 3 reports and what the rounds they count landed, so a continue is weighed against what the loop has taken rather than against how long it has run.
     - Describe each finding as its own verdict describes it: one you accepted is above the bar by that verdict, so calling it a preference or a matter of wording to strengthen the case for stopping misreports your own decision to the person deciding.
     - Reading those groups, name what the rounds could not settle for themselves and what would settle it, or say there is nothing.
     - Put to them what only they can settle; where nothing is left for them, put continuing alone.
       - Say what the rounds kept trying to do and whether what they produced serves it, before listing anything.
       - Then give the answers, what each costs, and which you recommend, including at least one no round produced; a menu built only from the rounds' own history keeps the decision inside the frame that generated the findings.
       - What their answer settled goes into the background as theirs, as a constraint on what the change may be; the reason they gave stays in the verdict section.
       - Read continuing out of their answer where they did not say it: one that settles the question and says nothing of going on is a continue under what it settled, narrowed as the message recommended; one that continues and settles nothing is a continue too, and the question it left goes to the close.
       - A settlement re-frames the round's held edits: re-weigh them against it before applying.
     - Recommend the endgame as the answer: continue, with a gate on what the round may apply.
       - It applies every fix a valid finding outside the floor lands, and nothing besides.
       - A fix the user directed in so many words is outside it: that is an instruction rather than an answer to a finding.
     - Their stop voids the round's other answers; the close names what it voided.
       - What they settled is not among them: it settles what the change may be rather than the round, and reaches the background and the close report either way.
     - Their continue governs every round that follows, restarting the tally rather than retiring the question.
       - Consecutive rounds are counted afresh from the continue.
       - What earlier rounds added keeps counting from the loop's start.
       - When the condition is met again, hold again.
   - Where the round needs the user, everything it needs them for goes to them together, in one message:
     - An answer this message waits on comes from the user and no one else, the loop included; the one thing it may take as given is the continue the hold above reads out of a settlement.
       - Where an answer the round cannot go on without does not come, record the held state — the questions, what each answer would change, the would-be verdicts — in the verdict section, leave the record unclosed, and end the run with the questions as its result.
     - A fix reaching past the purpose bound puts one question — extend the change, carve it out, or leave it.
       - Carve it out yourself, without waiting, where the excess separates cleanly — what stays behind is coherent and correct without it.
       - Tell the user in the round's message, so they can extend the change instead; a carve-out they only learn about at the close is work that quietly never happens.
         - Their extend pulls the carved-out part back into the round's work.
       - Wait where it does not separate cleanly.
     - A valid finding outside the floor that the loop cannot fix is put as carve it out or leave it, and waits for the answer.
     - Wait on a reviewer's objection to the pinned purpose, the first time it is raised — only the user can re-pin it.
       - Their answer goes into the background.
       - A re-pin re-frames the round's other questions: re-weigh them against it before acting on their answers.
     - Do not wait on a flawed spec or a contradicted prescribing claim: the loop is barred from fixing either whatever the answer.
       - Surface each once.
   - Anything the record disposed of and a later round raises again is answered from the record where it comes back with no evidence the record does not already answer, and judged afresh on its merits where it does, from the record's reasoning rather than its verdict.
     - It joins the settled list step 2 sends, and a loop spawning fresh reviewers produces recurrence by design, so recurrence is not evidence against the verdict whether or not the list had reached that reviewer.
     - Distinct findings clustering on one mechanism are `address-finding`'s same-mechanism signal, not recurrence.
     - A decision the user gave in so many words — an instruction, or their answer to a question put to them — is not re-judged; one read from their context is an inference and stays review surface.
   - A decision's justification lives in the record, and the loop does not also write it into the change: a sentence a reader who never saw this review would not need is there to answer the last round, and belongs in the record.
   - For `address-finding`'s no-silent-reversal check, the piece of work is the change under review together with the fixes and record this loop has accumulated, not the current round alone.
5. **Loop.** Spawn a fresh review subagent and repeat until a round applies no fix (putting in the reviewer's own proposal unaltered is a fix); a round can return findings and still be the last, so long as it changed nothing.
   - Termination is defined by applying no fix, so a loose fix bar puts it out of reach; a finding that would loosen the stopping rule rather than inspect the bar is answered from this decision.
   - What gates the close is what reviewers see: a round that edited the change (cuts included) or added to or corrected the background cannot be the last, since no reviewer has read the result.
     - Verdict writes, background cuts or condensations, and the facts a round established — each carrying what established it, so a reviewer checks rather than trusts it — never extend the loop.
       - Correcting the background likewise, where the correction leaves what a later reviewer would look into unchanged; one that moves it either way extends the loop, since no reviewer has read the change under it.
   - Close every round by looking at the change whole before deciding whether to spawn, on the last round as much as any other — though a stop-close reports rather than cuts:
     - whether the fixes so far read as one coherent thing rather than a stack of patches;
     - whether the rounds have piled up more than a developer should have to hold in mind — cut what they piled up before it becomes what the next round reviews;
     - the background and the settled list likewise, since both are sent to every reviewer and grow every round.
   - A cut this look makes is confined to what the rounds themselves put there, under `address-finding`'s removal question; a decision entry step 4 requires may be condensed, never dropped.
   - A cut to the change, or a consolidation, goes through `address-finding` like any finding, purpose bound included.
   - When the loop closes, report what the user has to read:
     - a valid finding outside the floor left unfixed, and why;
     - any decision that departed materially from what they asked for, or from a plan they approved;
     - a prescribing claim the fix contradicts;
     - what the last round raised outside the floor and you rejected;
     - every objection to a user decision the rounds set aside;
     - every question put to them that no answer came for, with what each answer would change;
     - a continue the loop read out of a settlement rather than one they stated;
     - carve-outs, flawed specs, what they settled, and the questions above, which outlive the loop — the record holds them but is no lasting home, so say so and let the user move them somewhere that is.
   - Close the record by appending `## Closed` as its last line rather than deleting it, which would take the carve-outs, flawed specs, and unanswered questions with it.
   - No fix lands under `## Closed`, whatever reopened the item and whatever the close reported it as — a rejection the user pushes back on, a question they answer late: what the close buys is that no fix ships unread, and an edit under `## Closed` spends that.
     - Reopen the record and let a round review the fix, or report the re-judging and leave the item.

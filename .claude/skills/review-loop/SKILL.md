---
name: review-loop
description: "Orchestrate an iterate-until-clean review of code you just changed: delegate the review to a subagent, judge each finding, then loop fix and re-review until a round applies no fix, reporting at the end what the user has to act on. Invoke when the user asks to pass the current changes through review, or when a workflow step calls for a review pass. The subagent does the actual review via the raise-findings skill and makes no edits."
---

# Review Loop

## The loop

A loop runs rounds of one shape: a fresh reviewer reads the change, the loop judges what it returned, and the round then does one of three things — it applies fixes, it closes, or it stops to ask the user.
"The round" below is that shape, and its last three parts are those three outcomes.
"The record" is what carries the loop across rounds and across a lost context; "Before the first round" is what has to be true before any of it runs.

## The record

The loop keeps its state in a file, so it survives a compaction and a session that ends mid-loop; re-read it whenever your own context no longer holds what it says.
It lives at `$(git rev-parse --path-format=absolute --git-common-dir)/review-loop/$(git rev-parse --abbrev-ref HEAD).md` (create the leading directories) — under the common git dir, which linked worktrees share and `git worktree remove` leaves alone, so it never reaches the reviewer through `git status`.

It has two sections, and the headings are bookkeeping that stays in the file:

- `## Background (sent to the reviewer)` — what is true of the change and why it has this shape, each reason carrying whose it is, covering what only this side knows and the diff does not show:
  - the pinned purpose;
  - the use the change is built for;
  - the constraints the deliverable must keep, a decision's outcome among them where it constrains what the change may be;
  - what the change deliberately does not cover;
  - what the rounds established about the change or what it runs against, each fact carrying what established it.
- `## Verdicts` — entries grouped by round, the group noting the fixes the round landed and what it added to the background, and each entry holding:
  - what was raised;
  - what was decided;
  - the reason, written to be weighed rather than taken on trust, and the alternatives a design choice was chosen over;
  - whether the loop or the user decided it.

The sections divide by reader: the background is written for the reviewer, the verdicts for this loop, and a ground both readers need may be owed to both.
The verdict section is never sent whole or quoted; what a reviewer needs from it travels in the settled list "Opening the round" derives.

What a round enters, and how the record ends:

- **Every outcome.** Enter it in the verdict section.
  - An outcome is anything the round disposed of or decided, whatever verdict it took and whether or not a finding forced it.
  - An entry has to be actable without redoing the round:
    - what each axis of a consistency sweep reached, or that the axis reached nothing;
    - what a continue narrowed the round to.
- **Closing the record.** Append `## Closed` as its last line rather than deleting it, which would take everything the close reports as carried over with it.
- **After the close.** No fix lands under `## Closed`, whatever reopened the item and whatever the close reported it as: what the close buys is that no fix ships unread, and an edit under `## Closed` spends that.
  - Reopen the record and let a round review the fix, or report the re-judging and leave the item.

## Before the first round

State what this change is for before the first round, and hold every later round to that statement — re-inferring it from a diff the rounds grew turns the bound into a ratchet.

- **Find the record first**, before anything else: this branch's path first, then — since a branch switch strands a record under the old name — the rest of `review-loop/`, at every depth, for an unmarked record whose content matches.
  - An unmarked record of this work at this branch's path is an interrupted loop: resume it, adding to its background rather than composing one over it.
  - A record whose last line is `## Closed` is a finished loop's: set it aside under a name that says which loop it was.
  - Anything else — a match under another branch's name, a record you cannot place — goes to the user before you touch it.
  - Nothing found means a fresh loop: create the record.
- **Where the purpose comes from.** Take it from the first of these that states one, and say which source it came from:
  - the user;
  - a resumed record;
  - the workflow that invoked this loop;
  - the diff, read as a whole.
- **State it as an outcome**, the one the change is for; where the background also names the files the change touches, that describes the change and does not narrow the bound.
- **Do not wait for a reply.** State it, with the rest of the background you composed, and carry on.
- **What the pinned purpose serves.** As the background currently holds it, it serves as `address-finding`'s purpose statement for every round; restate it only when putting an excess to the user, who is judging against it.
  - The constraints the background states go to it alongside, as part of the bar its fix discipline holds the fix to.
- **Fill the background** before the first round.
  - Enter inferences as inferences: a guess dressed as a user decision misleads every reviewer and would be shielded from re-judging.
- **Work already landed** is in-bound for later rounds, and licenses no further excursion past the purpose.

## The round

### Opening the round

Spawn a general-purpose subagent (the `Agent` tool) and, in its prompt, instruct it to review the current changes by running the `raise-findings` skill.

- `raise-findings` is a Skill, not an agent type — do NOT pass it as `subagent_type`, since that call fails.
- **The tree may not move.** However you spawn, the reviewer reads a working tree no one may move, since an edit landing under it attaches the round's findings and verdicts to a state that no longer exists: before this run's first spawn, tell a user you can reach not to move it until this run ends.
  - Note into the round's verdict group a digest of what the reviewer reads when you spawn, rather than a description of it, re-noting it after anything of your own writes there once you have judged.
  - Confirm that it still matches before you report the round's findings, before you judge them, and on any answer to a wait; where it does not, say so and discard them to spawn afresh, or report the mismatch with them where that answer was to stop.
  - Where you ended the turn on the spawn: resume on the notification from the reviewer you spawned last, discarding one from a reviewer you replaced rather than polling for either.
  - What the user sends while a round is in flight goes into the record before that round goes on: a correction to the background lands there in their own words, and an instruction goes into the round's verdict group, which the close reports where the loop left it undone.
    - What invalidates that round — what its reviewer read the change against, or a fix it has already landed — voids it, as does the tree moving before you judge, your own early application included: discard its findings and spawn afresh, stopping the reviewer first where one is still running.
    - Anything else is answered at once where it calls for no edit, and otherwise waits for the findings and is taken up once you have judged them; applying it sooner costs the round, so take that only where the user asks for it.
- **In its own session**, pass `run_in_background: true` and end your turn on the spawn rather than holding it open until the findings land.
- **Inside a subagent**, pass `run_in_background: false` and hold the turn: a subagent returns on ending its turn, and merely keeping the turn open gets it the spawn rather than the findings.
- **A reviewer that did not review** is replaced: spawn afresh inside the round.
- **The reviewer's model.** Choose it rather than letting it inherit, since inheritance takes the tier of whatever spawned it.
  - Spawn the reviewer on the `opus` tier, unless the user or the invoking workflow named a higher one for this reviewer.
  - Where the running model family offers no such tier, name the tiers it does offer and ask, rather than picking one.
- **What the reviewer may do.** It returns findings only, makes no edits, and runs nothing that writes the change under review.
- **The common git dir.** Bar it from touching that.
- **What the prompt carries.** Beyond those run constraints it carries four things, and nothing else out of the verdict section:
  - the changes, meaning the diff baseline to read them against and whatever no diff reaches, which `raise-findings`' own scoping section lists;
    - Check yourself, before each spawn, that the diff that baseline produces and whatever no diff reaches are this change and nothing else.
  - the background section's contents, copied whole;
  - the settled list this round derives from the verdict section, a line for each finding a round disposed of without landing a fix, giving what was raised and the ground it went on;
    - The ground is whatever disposed of the finding, the floor and an objection's setting-aside included, and not only a verdict on its merits.
    - Write each line as what happened rather than as a verdict to honour, and in terms its reader can weigh without this file: "raised in round 2, left at the floor as the reviewer's lowest rank", not "X is not a problem".
    - Head the list with its one condition: raise one of these again only with evidence the ground it went on does not already answer.
  - what this loop adds to what `raise-findings` asks for:
    - the purpose the background states bounds the fix and not the finding, so a defect is raised wherever the change has one;
    - where a finding is against a statement in the background, which of the reviewer's other findings rest on that statement, or that none do.
- **Nothing of your own.** Compose no review instructions on top.

Report the findings to the user as the subagent returned them.

- **What goes with the findings.** Say which round this is, counting from the loop's start, and the consecutive-round tally as "Waiting on the user" counts it, every round and not only when the hold fires, so a recurrence cannot pass unremarked.

### Judging the findings

Apply the `address-finding` skill (invoke it via the Skill tool) to judge each finding's validity, stating which you accept or reject and why.

- **Whose predicates.** They are that skill's, and this section points at them rather than restating one, so a correction lands where all its callers read it.
  - A narrowing the loop itself needs is not a restatement and stays here, as the re-raise handling below is, since this section is what those callers do not read.
- **Rank and verdict.** What a finding is ranked is the reviewer's, and the verdict is yours: a rank orders the round and never answers whether the finding holds.
- **The bar a fix clears.** It is `finding-bar`'s — or, where the finding is a removal, `finding-bar`'s removal bar, and where it is a standard violation, `address-finding`'s standard criterion — read off the deliverable and never off what the loop has spent. Where nothing clears that bar the round MUST apply nothing, which is a correct outcome and not a failed one.
- **The floor.** A finding it covers is not fixed for its own sake.
  - The floor covers the lowest rank of the reviewer's own scale, and a finding against the background on the same terms unless it shows a statement there false.
  - That falsity exception is not confined to the background, and puts the finding outside the floor rather than past the bar above: a statement the change made false or misleading is fixed at whatever rank it carries where a reader acts on it, and rejected where none does.
    - Read what the statement said before the change: one already wrong is a defect the change found rather than one it caused, so it leaves the floor under the purpose bound rather than past it.
  - A finding proposing a removal, or naming a violation of a written standard at whatever rank, sits outside the floor: judge and fix it for its own sake, whatever else the round lands.
    - Put the standard violation through `address-finding`'s standard criterion first, and reject the finding where it fails there, or a preference dressed as a rule buys its way past the floor.
  - Anything else a floored finding proposes is applied only where a fix the floor does not cover already edits what it names.
  - What is applied that way is checked against the purpose, the decisions already taken, and the sites it touches.
- **The probe.** Trying the text on a reader is the loop's to run, not the reviewer's: a finding claiming a reader acts wrongly under the text predicts a behaviour, and putting the text in front of one measures it.
  - Run it at your discretion, where argument has not settled the claim and the floor does not already cover it; no verdict waits on a probe, and a finding is accepted or rejected on argument where argument settles it.
  - Put the text where its reader would meet it, and a task its scenario calls for, to a fresh subagent, spawned synchronously, whose prompt bars it from changing anything or reading under the common git dir, and read the wrong act off what it produces.
  - Compose the task without the finding's framing or the reading it names — a reader handed the wrong reading takes it, and one asked about a sentence finds it.
  - A run that never engaged the task shows nothing and is replaced.
- **A finding against the background.** A valid one outside the floor, against the background rather than the change, is answered by correcting the background; never edit the change to make an argument come out right.
  - What the user stated in their own words — the purpose, a constraint, a question they settled — is not yours to rewrite: put the refutation to them in the round's message, without waiting, and their answer updates the background in their own words.
    - Enter it in the verdict section meanwhile.
    - An objection to the pinned purpose is the exception: it waits, as "Waiting on the user" says.
- **Re-raising.** Anything the record disposed of and a later round raises again is answered from the record where it comes back with no evidence the record does not already answer, and judged afresh on its merits where it does, from the record's reasoning rather than its verdict.
  - A loop spawning fresh reviewers produces recurrence by design, so recurrence is not evidence against the verdict.
  - Distinct findings clustering on one mechanism are `address-finding`'s same-mechanism signal, not recurrence.
- **The user's own decisions.** One given in so many words — an instruction, or their answer to a question put to them — is not re-judged, so an objection to the choice itself is set aside unjudged; one read from their context is an inference and stays review surface.
- **The scope of no-silent-reversal.** For `address-finding`'s check, the piece of work is the change under review together with the fixes and record this loop has accumulated, not the current round alone.

### Choosing what the round does

Look at the change whole before deciding which of the three the round takes, on the last round as much as any other — though a stop-close reports rather than cuts:

- whether the fixes so far are one coherent thing rather than a stack of patches, which `address-finding`'s symptom-treating signals decide and not how the result reads;
- whether the change has piled up more than a developer should have to hold in mind — you MUST cut what piled up before it becomes what the next round reviews;
  - Read its net line change against the baseline, counting whatever no diff reaches, before judging that by eye, since the question is the total a developer holds and the eye only ever meets one round's addition.
- the background likewise, and the verdict entries the settled list derives from.

A cut this look makes to the background or the verdict entries is confined to what the rounds themselves put there, under `address-finding`'s removal question; a decision entry the record requires may be condensed, never dropped.
A cut to the change, or a consolidation, goes through `address-finding` like any finding, purpose bound included.

### Fixing

Fix the valid ones with `address-finding`, then spawn a fresh reviewer and run the next round.

- **Facts into the background.** A fact a round established rather than argued — `address-finding`'s work to settle a fix included — MUST go into the background in the record, and not only into the prompt that copies it, so the next reviewer reads it rather than deriving it again; whatever verdict it served stays in the verdict section.
  - Each such fact carries what established it — the command and what it returned, the test that covers the behavior, the probe's task — or a later reviewer cannot tell whether its own concern is the one that was answered.
  - A reviewer's argument is not one of those: establish it yourself before entering it as one — run the command, run the test, run the probe, or read the text — since what the background states is read as settled by every round after and none of them can see it was never checked.
- **Justification.** A decision's justification lives in the record, and the loop MUST NOT also write it into the change: a sentence a reader who never saw this review would not need is there to answer the last round, and belongs in the record.

### Closing

The loop closes on a round that applies no fix (putting in the reviewer's own proposal unaltered is a fix); a round can return findings and still be the last, so long as it changed nothing.

- **What termination is.** It is defined by applying no fix, so a loose fix bar puts it out of reach; a finding that would loosen the stopping rule rather than inspect the bar is answered from this decision.
- **The gate.** What gates the close is what reviewers see: a round that edited the change (cuts included) or added to or corrected the background cannot be the last, since no reviewer has read the result.
  - Verdict writes, background cuts or condensations, and the facts a round established — each carrying what established it, so a reviewer checks rather than trusts it — never extend the loop.
    - Correcting the background likewise, where one of the two below holds.
      - The correction leaves what a later reviewer would look into unchanged.
      - The round's reviewer is what raised the problem, the correction is not the user's own words but the loop's answer to that finding and no more, and the reviewer reported that none of its other findings rests on the statement it struck.
- **The report.** When the loop closes, report what the user has to act on — the behaviour the rounds added, altered or removed, and what the loop proposes to carry over.

### Waiting on the user

Where the round needs the user, everything it needs them for goes to them together, in one message.

- **The answer is theirs alone.** It comes from the user and no one else, the loop included; the one thing this message may take as given is the continue the hold below reads out of a settlement.
  - **Where none comes**, record the held state — the questions, what each answer would change, the would-be verdicts — in the verdict section, leave the record unclosed, and end the run with the questions as its result.
- **What a settlement of theirs does.** It reaches the background in their own words, as a constraint on what the change may be, whether or not a question put it to them.
  - **Re-weigh before acting.** A settlement re-frames what the round was about to do, its held edits and its other questions alike, so weigh those against it rather than acting on the answers as they were framed.

The hold, which stops a round that is refining the loop's own work:

- **The trigger.** Before applying any edit you MUST read the verdict groups: when consecutive rounds' edits have landed mainly on what earlier rounds added — in the change or the background — and this round's accepted findings outside the floor land there again, the loop is refining its own additions: hold the round's edits, put the round's open question to the user in the round's message, and wait.
- **What the two answers mean.** Continue means apply and go on; stop means close with the held findings reported as left unfixed — so no fix ever ships unreviewed.
- **What the message carries.** The counts "Opening the round" reports and what the rounds they count landed, so a continue is weighed against what the loop has taken rather than against how long it has run.
  - **Describe each finding as its own verdict describes it.** One you accepted is above the bar by that verdict, so calling it a preference or a matter of wording to strengthen the case for stopping misreports your own decision to the person deciding.
  - **The question.** Name what the rounds could not settle for themselves and what would settle it, and put that to them; where nothing is left for them, say so and put continuing alone.
  - **The menu.** Say what the rounds kept trying to do and whether what they produced serves it, before listing anything.
    - Then give the answers, what each costs, and which you recommend, including at least one no round produced; a menu built only from the rounds' own history keeps the decision inside the frame that generated the findings.
    - Recommend the endgame: continue, with a gate that applies every fix a valid finding outside the floor lands, and nothing besides.
      - A fix the user directed in so many words is outside it: that is an instruction rather than an answer to a finding.
- **Reading their answer.** One that settles the question and says nothing of going on is a continue under what it settled, narrowed as the message recommended; one that continues and settles nothing is a continue too, and the question it left goes to the close.
  - Their stop voids the round's other answers and the close names what it voided; what they settled is not among them, since it settles what the change may be rather than the round.
  - Their continue governs every round that follows, restarting the tally rather than retiring the question, though what earlier rounds added keeps counting from the loop's start.

What else the round waits on, or surfaces without waiting:

- **A fix reaching past the purpose bound** puts one question — extend the change, carve it out, or leave it.
  - Carve it out yourself, without waiting, where the excess separates cleanly — what stays behind is coherent and correct without it.
  - Tell the user in the round's message, so they can extend the change instead; a carve-out they only learn about at the close is work that quietly never happens.
    - Their extend pulls the carved-out part back into the round's work.
  - Wait where it does not separate cleanly.
- **A valid finding outside the floor that the loop cannot fix** is put as carve it out or leave it, and waits for the answer.
- **A fix whose worth turns on whether its situation arises** is NEVER yours to settle: leave it, and surface it in the round's message so they can say otherwise.
  - Running the mechanism establishes that it works, never that anyone reaches it, and the deployment is theirs to know.
- **What stays outside the change**, whatever left it there, MUST go into the background as what it does not cover.
- **An objection to the pinned purpose** waits, the first time it is raised — only the user can re-pin it.
- **A flawed spec, or a prescribing claim the fix contradicts**, waits on nothing: the loop is barred from fixing either whatever the answer. Surface each once.

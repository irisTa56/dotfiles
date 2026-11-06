---
name: review-loop
description: "Orchestrate an iterate-until-clean review of code you just changed: a subagent reviews, you judge and fix, and rounds repeat until one leaves the change untouched. Invoke when the user asks to pass the current changes through review, or when a workflow step calls for a review pass."
---

# Review Loop

## The loop

A loop runs rounds of one shape: a fresh reviewer reads the change, the loop judges what it returned, and the round then does one of three things — it applies fixes, it proposes the close, or it stops to ask the user.
The last two both go to the user, and "Waiting on the user" governs both.
"The round" below is that shape, and its last three parts are those three outcomes.
"The record" is what carries the loop across rounds and across a lost context; "Before the first round" is what has to be true before any of it runs.

## The record

The loop keeps its state in files, so it survives a compaction and a session that ends mid-loop; re-read them whenever your own context no longer holds what they say.
Its verdict section lives at `$(git rev-parse --abbrev-ref HEAD).md` under "the record directory", `$HOME/.claude/review-loop/$(git rev-parse --path-format=absolute --git-common-dir | tr / -)/`, and its background at the same path under "the background directory", `$HOME/.claude/review-loop-background/` in place of `$HOME/.claude/review-loop/` (create the leading directories).
Each directory is keyed by the common git dir, which linked worktrees share, and sits outside the repository, so `git worktree remove` leaves it alone, it never reaches the reviewer through `git status`, and a worktree-isolated agent can still write it where the harness refuses that agent writes under the common git dir.

It has two sections, each a file of its own, so a reviewer can be handed the background without the verdicts:

- The background — what is true of the change and why it has this shape, each reason carrying whose it is, covering what only this side knows and the diff does not show:
  - the pinned purpose;
  - the use the change is built for;
  - the constraints the deliverable must keep, a decision's outcome among them where it constrains what the change may be;
  - what the change deliberately does not cover;
  - what the rounds established about the change or what it runs against, each fact carrying what established it.
- The verdict section, under a `## Verdicts` heading — the title the change would go up under, the baseline the rounds read the change against, and entries grouped by round, the group noting the fixes the round landed and what it wrote into the background, and each entry holding:
  - what was raised;
  - what was decided;
  - the reason, written to be weighed rather than taken on trust, and the alternatives a design choice was chosen over;
  - whether the loop or the user decided it.

The sections divide by reader: the background is written for the reviewer, the verdicts for this loop, and a ground both readers need may be owed to both.
The verdict section is never sent whole or quoted to a reviewer; what one needs from it travels in what "Opening the round" derives.
The close's selector, in [references/closing.md](references/closing.md), is the one reader that gets it whole.

What a round enters:

- **Every outcome.** Enter it in the verdict section.
  - An outcome is anything the round disposed of or decided, an answer the user gave it included, whatever verdict it took and whether or not a finding forced it.
  - An entry has to be actable without redoing the round, which for a consistency sweep means saying what each axis reached, or that the axis reached nothing.

## Before the first round

State what this change is for before the first round, and hold every later round to that statement — re-inferring it from a diff the rounds grew turns the bound into a ratchet.

Before the first round, and on resuming a loop, read [references/first-round.md](references/first-round.md) and do what it says: it finds the record, pins the diff baseline, the purpose and the title, and fills the background.

- **What the pinned purpose serves.** As the background currently holds it, it serves as `address-finding`'s purpose statement for every round; restate it only when putting an excess to the user, who is judging against it.
  - The constraints the background states go to it alongside, as part of the bar its fix discipline holds the fix to.
- **Work already landed** is in-bound for later rounds, and licenses no further excursion past the purpose.

## The round

### Opening the round

Spawn a general-purpose subagent (the `Agent` tool) and, in its prompt, instruct it to review the current changes by running the `raise-findings` skill.

- `raise-findings` is a Skill, not an agent type — do NOT pass it as `subagent_type`, since that call fails.
- **The tree may not move.** However you spawn, the reviewer reads a working tree no one may move, since an edit landing under it attaches the round's findings and verdicts to a state that no longer exists: before this run's first spawn, tell a user you can reach not to move it until this run ends.
  - Note into the round's verdict group a digest of what the reviewer reads when you spawn, rather than a description of it, re-noting it after anything of your own writes there once you have judged.
  - Confirm that it still matches before you report the round's findings, before you judge them, and on any answer to a wait; where it does not, say so and discard them to spawn afresh, or report the mismatch with them where that answer was to stop.
  - Where you ended the turn on a spawn: resume on the notification from the subagent you spawned last, discarding one from a reviewer you replaced rather than polling for either.
  - What the user sends while a round is in flight goes into the record before that round goes on: a correction to the background lands there in their own words, and an instruction goes into the round's verdict group, which the close reports where the loop left it undone.
    - What invalidates that round — what its reviewer read the change against, or a fix it has already landed — voids it, as does the tree moving before you judge, your own early application included: discard its findings and spawn afresh, stopping the reviewer first where one is still running.
    - Anything else is answered at once where it calls for no edit, and otherwise waits for the findings and is taken up once you have judged them; applying it sooner costs the round, so take that only where the user asks for it.
- **The wait.** Spawn with no `run_in_background`, and where the call returns a launch, end your turn without delivering a result rather than holding it open until the findings land: the reviewer's report resumes you, and a subagent's run ends on delivering its result, so delivering it on the launch gets its caller the spawn rather than the findings.
- **A reviewer that did not review** is replaced: spawn afresh inside the round.
- **The reviewer's effort.** Pass `effort: high`, whatever effort this session runs at, unless the user or the invoking workflow named another for this reviewer.
- **What the reviewer may do.** It returns findings only, makes no edits, and runs nothing that writes the change under review.
- **The common git dir and the record directory.** Bar it from touching either.
- **What the prompt carries.** Beyond the skill it names for the reviewer and those run constraints, it carries three things, each taken from a source outside this file, and nothing else out of the verdict section, plus one line where this round reads much of the change afresh:
  - the changes, meaning the diff baseline to read them against and whatever no diff reaches, which `raise-findings`' own scoping section lists;
    - Check yourself, before each spawn, that the diff that baseline produces and whatever no diff reaches are this change and nothing else.
    - Send the baseline the record pinned, whatever the rounds have committed since.
  - the background file's path, for it to read whole;
  - the settled list this round derives from the verdict section, a line for each finding a round disposed of without landing a fix, giving what was raised and the ground it went on;
    - The ground is whatever disposed of the finding, the floor and an objection's setting-aside included, and not only a verdict on its merits.
    - Write each line as what happened rather than as a verdict to honour, and in terms its reader can weigh without this file: "raised in round 2, left at the floor: the change is built for a base one person reads, so a second confirmation costs more than the state it catches", not "X is not a problem".
  - on a round that reads much of the change afresh, the line asking for the built-in reviews `raise-findings` names, as the loop's first round does and as one does after fixes that added or rewrote a large part of the change, or a settlement that re-pinned the purpose.
- **Nothing of your own.** Compose no review instructions on top, and put none on that list.

Report the findings to the user as the subagent returned them, and take the round on to its outcome in the same turn, or the next where a subagent it spawned ends this one on a launch.

- **What goes with the findings.** Say which round this is, counting from the loop's start, and the consecutive-round tally as "Waiting on the user" counts it, every round and not only when the hold fires, so a recurrence cannot pass unremarked.

### Judging the findings

Apply the `address-finding` skill (invoke it via the Skill tool) to judge each finding's validity, stating which you accept or reject and why.

- **Whose predicates.** They are that skill's, and this section points at them rather than restating one, so a correction lands where all its callers read it.
  - A narrowing the loop itself needs is not a restatement and stays here, as the re-raise handling below is, since this section is what those callers do not read.
- **The bar a fix clears.** It is `finding-bar`'s — or, where the finding is a removal, `finding-bar`'s removal bar, and where it is a standard violation, `address-finding`'s standard criterion — read off the deliverable and never off what the loop has spent. Where nothing clears that bar the round MUST apply nothing, which is a correct outcome and not a failed one.
- **The floor.** A valid finding is not fixed for its own sake where the use the change is built for makes it excessive; that judgement is yours.
  - Name in the round's message every finding you floored and the use you weighed it against, so the user can say otherwise.
  - Weigh it against the use the background states, asking first whether the condition the finding names arises in that use; the change's own text describing the condition is not evidence that it does, and where nothing the user said or the background states shows it, that question is not yours, as [references/waiting.md](references/waiting.md) says.
  - A finding you disagree with on its merits is answered on the bar and rejected, which leaves nothing to ride along.
  - Nothing floors a finding proposing a removal, or one against the background: judge and fix it for its own sake, whatever else the round lands.
  - Anything else a floored finding proposes is applied only where a fix the floor does not cover already edits what it names.
  - What is applied that way is checked against the purpose, the decisions already taken, and the sites it touches.
- **The probe.** Before accepting a finding whose harm is a reader act nothing has yet produced, run the probe [references/probe.md](references/probe.md) describes.
- **A finding against the background.** A valid one against the background rather than the change is answered by correcting the background; never edit the change to make an argument come out right.
  - What the user stated in their own words — the purpose, a constraint, a question they settled — is not yours to rewrite: put the refutation to them in the round's message, without waiting, and their answer updates the background in their own words.
    - Enter it in the verdict section meanwhile.
    - An objection to the pinned purpose is the exception: it waits, as [references/waiting.md](references/waiting.md) says.
- **Re-raising.** Anything the record disposed of and a later round raises again is answered from the record where it comes back with no evidence the record does not already answer, and judged afresh on its merits where it does, from the record's reasoning rather than its verdict.
  - A loop spawning fresh reviewers produces recurrence by design, so recurrence is not evidence against the verdict.
  - Distinct findings clustering on one mechanism are `address-finding`'s same-mechanism signal, not recurrence.
- **The user's own decisions.** One given in so many words — an instruction, or their answer to a question put to them — is not re-judged, so an objection to the choice itself is set aside unjudged; one read from their context is an inference and stays review surface.
- **The scope of no-silent-reversal.** For `address-finding`'s check, the piece of work is the change under review together with the fixes and record this loop has accumulated, not the current round alone.

### Choosing what the round does

Look at the change whole before deciding which of the three the round takes, on the last round as much as any other — though a stop-close reports rather than acts on the change:

- whether the fixes so far are one coherent thing rather than a stack of patches, which `address-finding`'s symptom-treating signals decide and not how the result reads;
- whether what the rounds have added has taken the change past the title the record holds;
  - What naming it now would need a clause for is past what the change is for: carve that out where it separates cleanly, saying so in the round's message, and where it does not, put it to them and wait.
  - What the rounds have grown past the title is what to re-decide the shape for: what they have established since is what the change did not have when it took its shape, so ask what shape it would take decided now, knowing that. Cutting alone leaves the design the parts sat in untouched.
- the background and the verdict entries the settled list derives from, on whether they still hold only what their reader needs.

Where a question of this look's reaches a decision of the user's own, what it would have asked goes to them at once and the round waits, as an unfixable finding's does, in the terms the look asked it in rather than being set aside unjudged.
A cut this look makes to the background or the verdict entries is confined to what the rounds themselves put there, under `address-finding`'s removal question; a decision entry the record requires may be condensed, never dropped.
A cut to the change, a consolidation, or a reshape goes through `address-finding` like any finding, purpose bound included.

### Fixing

Fix the valid ones with `address-finding`, then spawn a fresh reviewer and run the next round.

- **Facts into the background.** A fact a round established rather than argued — `address-finding`'s work to settle a fix included — MUST go into the background file, so the next reviewer reads it rather than deriving it again; whatever verdict it served stays in the verdict section.
  - Each such fact carries what established it — the command and what it returned, the test that covers the behavior, the probe's task — or a later reviewer cannot tell whether its own concern is the one that was answered.
  - A reviewer's argument is not one of those: establish it yourself before entering it as one — run the command, run the test, run the probe, or read the text — since what the background states is read as settled by every round after and none of them can see it was never checked.
- **Justification.** A decision's justification lives in the record, and the loop MUST NOT also write it into the change: a sentence a reader who never saw this review would not need is there to answer the last round, and belongs in the record.

### Closing

The loop closes on a round that left the change untouched; it can return findings and still be the last, and what it did to the record does not enter.
The loop proposes that close and the user makes it.

Before proposing it, read [references/closing.md](references/closing.md) and do what it says: what the close carries, who selects the carry-over and how its selection is checked, and how the record is closed.

### Waiting on the user

Where the round needs the user, everything it needs them for goes to them together, in one message.

- **The answer is theirs alone.** It comes from the user and no one else, the loop included; the one thing this message may take as given is the continue the hold below reads out of a settlement.
  - **Where none comes**, record the held state — what was put to them, what each answer would change, the would-be verdicts — in the verdict section, leave the record unclosed, and end the run with the held state as its result.

When the user answers, or a round meets any of these, read [references/waiting.md](references/waiting.md) before acting on it:

- a fix reaching past the purpose bound;
- a valid finding outside the floor that the loop cannot fix;
- a fix whose worth turns on whether its situation arises;
- something staying outside the change;
- an objection to the pinned purpose;
- a flawed spec, or a prescribing claim the fix contradicts.

The hold, which stops a round that is refining the loop's own work:

- **The trigger.** Before applying any edit you MUST read the verdict groups: when consecutive rounds' edits have landed mainly on what earlier rounds added — in the change or the background — and this round's accepted findings outside the floor land there again, the loop is refining its own additions: hold the round's edits, put the round's open question to the user in the round's message, and wait.
- **What the two answers mean.** Continue means apply and go on; stop means close with the held findings reported as left unfixed — so no fix ever ships unreviewed.
  - Their continue governs every round that follows, restarting the tally rather than retiring the question, though what earlier rounds added keeps counting from the loop's start.

When the hold fires, read [references/hold.md](references/hold.md) for what its message carries and how to read the answer.

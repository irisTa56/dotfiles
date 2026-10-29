---
name: measurement-design
description: Design a measurement whose numbers will decide something — a benchmark, a performance or configuration comparison, a load test, a capacity or ceiling check — before it runs, and draw conclusions from its results. Use whenever planning such a run, whether you or a person will run it, whenever choosing what to run next after seeing a result, and whenever saying what measured numbers show, even when nobody says "experiment" or "benchmark".
---

# Designing a measurement

A number counts only if a reader who was not in the conversation reaches the same conclusion from it.
Everything below serves that reader: they cannot ask which step produced the number, what else changed, or why this run and not another.

When a person executes the run from a page, the page carries every part of the design below that the executor acts on or records, and is written with the `runbook` skill.
Write the prose of a plan by [document-writing.md](../../rules/document-writing.md), and when it is in Japanese, compose it with the `japanese-tech-writing` skill as well.

## Fix the design before the run

- Write the procedure down before running it.
  - A number from a step that was not written down is not data. Note it as what changed the plan, and keep it out of the results.
- Fix in advance what each outcome leads to, with the threshold a difference must clear to count.
  - Choosing what to run next from a result is not a procedure: the choice bends towards the hypothesis, and a reader cannot tell which runs were planned.
  - A result that fits none of the fixed branches sends you back to the design, not on to an improvised run.
  - Results already in hand when no branches were fixed, such as numbers someone pastes or a run you just made, are exploration: they may suggest the next design, but that design fixes its branches and threshold before its own runs, and only those runs count as evidence for it.
- Fix in advance what voids a run, such as the load generator reaching its own limit, a service restarting mid-run, or a build other than the one planned, and have each run record what shows it. A voided run is repeated, not read.
- Keep an instrument that the no-difference branch would need, such as a counter showing the changed path was exercised at all. Without it, "no difference" cannot be told apart from "the change never took effect".
- State the assumption behind a time estimate, such as the time per iteration and where that figure came from, so whoever plans around the estimate can see when it no longer holds.

## Choose what to vary

- Vary one factor per condition, so that a difference has one cause.
  - A change that is going out anyway does not ride along with the factor under test; give it a condition of its own or leave it out.
- Vary only factors that have a counterpart in the system the decision is about: a setting it can be given, a version it can run.
  - A knob that exists only in the test setup produces a number nobody can act on.
- Re-run the baseline in the same sequence as the conditions rather than reusing an earlier one, and alternate the conditions within the sequence rather than running each as one block. Load from elsewhere, warm caches and versions drift over time, and whatever drift lines up with one condition becomes part of the difference.

## Choose what to measure

- Prefer a quantity read directly over one inferred from data already at hand: a counter the system keeps, or a ceiling measured with the same tool that measures the demand.
  - An inference stacks assumptions the reader has to accept, and a direct reading needs none.
- A vendor's published figure is not evidence about this environment. Where the decision rests on a ceiling, measure it here.

## Size the run

- Size the run to the effect it has to resolve: enough repeats that the spread between them is small next to the threshold.
- Fix the number of repeats before the run, and do not add repeats after seeing the spread: stopping where the result looks settled is choosing from a result.
- Keep the repeat count the same across every condition the plan compares, rounded up from what the sizing needs to a round number. The same count lets runs compare directly, and a round one shows the reader it was set in advance rather than where the runs happened to stop.

## Draw the conclusion

- Attribute a difference to a factor only where the runs compared differ in that factor alone and come from the same sequence. Where the runs at hand do not, say what they cannot show, and name the run that would show it.
- Give a quantity as the measured number or range, with the number of runs behind it, rather than as an adjective such as "faster" or "about the same".
- Read the result through the branches fixed before the run, and say which one it fell in.

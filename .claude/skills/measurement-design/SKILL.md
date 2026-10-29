---
name: measurement-design
description: Design a measurement whose numbers will decide something — a benchmark, a performance or configuration comparison, a load test, a capacity or ceiling check — before it runs, and draw conclusions from its results. Use whenever planning such a run, whether you or a person will run it, whenever choosing what to run next after seeing a result, and whenever saying what measured numbers show about a comparison or a decision, even when nobody says "experiment" or "benchmark".
---

# Designing a measurement

A number counts only if a reader who was not in the conversation reaches the same conclusion from it.
Everything below serves that reader: they cannot ask which step produced the number, what else changed, or why this run and not another.

The sections follow the measurement through design, run and reading.
Every decision is taken in the first, before the first measured run, and the only run allowed before the design is final is a pilot it names: the later two say how the run and the reading use what the design fixed, so a design that leaves one of them open has left a choice to be made after seeing a result.

Write the design down before running it; a number from a step that was not written down is not data.
When a person executes the run from a page, that page carries every part of the design the executor acts on or records, and is written with the `runbook` skill.
Write the prose of a plan by [document-writing.md](../../rules/document-writing.md), and when it is in Japanese, compose it with the `japanese-tech-writing` skill as well.

## Design

### What to vary

- Vary one factor per condition, so that a difference has one cause.
  - A change that is going out anyway does not ride along with the factor under test; give it a condition of its own or leave it out.
- Vary only factors that have a counterpart in the system the decision is about: a setting it can be given, a version it can run.
  - A knob that exists only in the test setup produces a number nobody can act on.

### What to measure

- Prefer a quantity read directly over one inferred from data already at hand: a counter the system keeps, or a ceiling measured with the same tool that measures the demand.
  - An inference stacks assumptions the reader has to accept, and a direct reading needs none.
- A vendor's published figure is not evidence about this environment. Where the decision rests on a ceiling, measure it here.
- Keep an instrument that the no-difference branch would need, such as a counter showing the changed path was exercised at all.
  - Without it, "no difference" cannot be told apart from "the change never took effect".

### How the conditions are run

- Re-run the baseline in the same sequence as the other conditions rather than reusing an earlier result.
  - Load from elsewhere, warm caches and versions drift over time, so a baseline from another session carries that drift into the difference.
- [Interleave the conditions](https://research.spec.org/icpe_proceedings/2017/proceedings/p287.pdf) within the sequence, in a shuffled order each round, rather than running each condition as one block.
  - Draw the order for every round before the first run and write it into the design, so the run follows it and a reader can see it.
  - Drift within a session lines up with whichever condition runs during it, so blocks still let it into the difference.
- Decide how [warm-up is handled](https://blog.acolyer.org/2017/11/06/statistically-rigorous-java-performance-evaluation/), such as discarding the first runs after a restart or measuring start-up as a quantity of its own, and handle it the same way in every condition.

### How many runs

- Size the run to the effect it has to resolve: enough repeats that the uncertainty of the difference between conditions is small next to the threshold.
  - Take the run-to-run spread this needs from earlier runs, or from a pilot run as a step of its own before the design is finished. Where there is neither, say that the count is a guess.
  - Either way the count goes into the design as a number before the first measured run. A count that a check may still change, such as "10, or 20 if the spread is wide", is not yet a count: run the check while writing the design and write its outcome.
  - A pilot's runs are exploration and are not pooled with the measured ones.
- Give every compared condition the same repeat count: the largest any of them needs, rounded up to a round number.
  - The same count lets runs compare directly.
  - A round count shows the reader it was set in advance rather than where the runs happened to stop.
- State the assumption behind a time estimate, such as the time per run and where that figure came from, so whoever plans around the estimate can see when it no longer holds.

### What the results will decide

- Fix what each outcome leads to, with the threshold a difference must clear to count.
- Fix what makes a run unusable, such as the load generator reaching its own limit, a service restarting mid-run, or a build other than the one planned, and what each run records to show it.
- Fix how many times a condition may be re-run to replace unusable runs before the measurement stops, and where in the interleaved order a replacement goes, the last round included.

## Run

- Run what the design lists and nothing else.
  - A result never adds a run, a repeat or a condition, since a choice made from a result bends towards the hypothesis and a reader cannot tell which runs were planned. What a result suggests goes into the next design.
- Replace an unusable run with a new one where the design placed replacements, rather than reading it, and record it with its cause, per condition.
- Stop at the re-run limit, and take the design back up rather than running on.
  - A condition whose runs keep becoming unusable may be showing what that condition does, such as the new setting making the service restart. Report how often each condition's runs were unusable as a result in its own right, rather than replacing the runs until that disappears.

## Reading

- Read the result through the outcomes fixed before the run, and say which one it fell in. A result that fits none of them sends you back to the design.
- Attribute a difference to a factor only where the runs compared differ in that factor alone and come from the same sequence.
  - Where the runs at hand do not, say what they cannot show, and name the run that would show it.
- Give a quantity as the measured number or range, with the number of runs behind it, rather than as an adjective such as "faster" or "about the same".
- Treat results gathered with no outcomes fixed beforehand, such as numbers someone pastes or a run you just made, as exploration.
  - They may suggest the next design, but only that design's own runs count as evidence for it.

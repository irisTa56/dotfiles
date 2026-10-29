---
name: measurement-design
description: Design a measurement whose numbers will decide something — a benchmark, a performance or configuration comparison, a load test, a capacity or ceiling check — before it runs, and draw conclusions from its results. Use whenever planning such a run, whether you or a person will run it, whenever choosing what to run next after seeing a result, and whenever saying what measured numbers show about a comparison or a decision, even when nobody says "experiment" or "benchmark".
---

# Designing a measurement

A number counts only if a reader who was not in the conversation reaches the same conclusion from it.

Take every decision in the design, before the first measured run.
The run and the reading then use what the design fixed, and anything it leaves open becomes a choice made after seeing a result.
Write the design down; a number from a step that was not written down is not a result the measurement can report.

When a person executes the run from a page, write that page with the `runbook` skill.
Write the prose of a plan by [document-writing.md](../../rules/document-writing.md), and when it is in Japanese, compose it with the `japanese-tech-writing` skill as well.

## Design

### What to vary

- Vary one factor per condition. A change that is going out anyway gets a condition of its own or stays out.
- Vary only factors that have a counterpart in the system the decision is about, such as a setting it can be given or a version it can run.

### What to measure

- Prefer a quantity read directly, such as a counter the system keeps or a ceiling measured with the tool that measures the demand, over one inferred from data already at hand.
- A vendor's published figure is not evidence about this environment.
- Keep an instrument that the no-difference branch would need, such as a counter showing the changed path was exercised, so "no difference" can be told apart from "the change never took effect".

### How the conditions are run

- Re-run the baseline in the same sequence as the other conditions rather than reusing an earlier result.
- [Interleave the conditions](https://research.spec.org/icpe_proceedings/2017/proceedings/p287.pdf) in a shuffled order each round rather than running each as one block, and write the order for every round into the design before the first run.
- Decide how [warm-up is handled](https://dri.es/files/oopsla07-georges.pdf), and handle it the same way in every condition.

### How many runs

- Size the run so that the uncertainty of the difference between conditions is small next to the threshold, taking the run-to-run spread from earlier runs or a pilot.
- Write the count into the design as one number. "10 runs, or 20 if the first runs scatter widely" is not yet a count, since whoever runs it then picks the count from results. Where no spread is known, the guess is the count: tell whoever approves the design that it is a guess, and keep that caveat off the executor's page.
- Give every compared condition the same repeat count: the largest any of them needs, rounded up to a round number, so the reader sees it was set in advance rather than where the runs stopped.
- State what a time estimate assumes, such as the time per run.

### What the results will decide

- Fix what each outcome leads to, the threshold a difference must clear, and what has to clear it: the interval of the difference for a stated statistic, built in a stated way, not the point estimate alone.
- Fix what makes a run unusable, such as the load generator reaching its own limit or a service restarting mid-run, and have the check that runs after each run decide it, record it with its cause, and exit non-zero, so nobody judges a printed verdict; the next scheduled run goes ahead.

## Run

- Run what the design lists and nothing else. A result never adds a run, a repeat or a condition; what it suggests goes into a revision of the design, written before any further run, which can be a few lines added to this one. The runs under a revision are read on their own against its outcomes, not pooled with the runs that prompted it.
- An unusable run is not read and not replaced. Report each condition's usable and unusable counts, since a condition whose runs keep failing may be showing what that condition does; where too few runs are usable to decide, that goes into a revision.

## Reading

- Say which outcome fixed before the run the result fell in. A result that fits none sends you back to the design.
- Attribute a difference to a factor only where the runs compared differ in that factor alone and come from the same sequence. Otherwise say what they cannot show, and name the run that would show it.
- Give a quantity as the measured number or range with the number of runs behind it, and a difference with its interval, rather than an adjective such as "faster".
- Treat results gathered with no outcomes fixed beforehand, such as numbers someone pastes, a pilot, or a run you just made, as exploration: they may suggest the next design, but only its own runs count as evidence for it.

---
name: test-drive
description: "Build or change behavior by test-driven development as Kent Beck describes it: list the behaviors, then, one behavior at a time, write its test, watch it fail, and write the code that passes it. Use whenever implementing a feature, changing what existing code does, or fixing a bug in a codebase that has automated tests or should have them, including when the request says nothing about tests."
---

# Test Drive

Build the change by test-driven development as Kent Beck describes it in [Canon TDD](https://newsletter.kentbeck.com/p/canon-tdd).
Write each test just before the code it asks for, and write no production code a failing test has not asked for.
A test written after the code mirrors what that code happens to do, so it passes against the code's own bugs and breaks when the inside is restructured.
One written first states what a caller needs, and has been seen to fail without it.

## The loop

1. List the behaviors the change has to have, a behavior being one thing a caller of the code can observe.
   - The list is a plan and not yet tests, and it grows as the work turns up cases.
2. Turn exactly one item into a test, or, where the work changes what existing code does, rewrite the tests that pin the old behavior so that they state the new one.
3. Run it and watch it fail, and read the failure: it should be the behavior that is missing.
   - A test that passes at once has pinned nothing the code lacked, so find out why before going on.
4. Write the least code that makes it and every earlier test pass.
5. Restructure only while the suite is green, then go back to step 2 until the list is empty.

Do not turn several items into tests ahead of their code.
Tests written in a batch describe the design as imagined before any of it ran, so the later ones cannot learn from what the earlier ones showed.

## Where a test attaches

- Where a spec or a plan has settled what is verified and at which boundary, follow it without asking for it again, and raise it only where you object to what it settled.
- Where none has, take both from the user's instruction: the behavior it describes, at the boundary its caller reaches it through.
- Ask only where the instruction leaves open something that changes what gets built, and do not stop partway through to confirm what it settles.
- Test a module through what it offers its callers, and give no test of its own to a function the module keeps private, such as a `defp` in Elixir.
  - The public function that uses it is where its behavior is tested, which keeps the tests few and lets the inside be restructured without rewriting them.
- Replace with a test double only what lies outside the system, such as the network, the clock, or a third-party service, and let the project's own modules, and its own database in a test instance, run for real.
  - A double standing in for the project's own code pins how its pieces call each other, so the test passes while the pieces disagree and fails when they are rearranged.

## Property-based testing for a law over a whole input domain

Where the behavior is a law that holds over a whole input domain, such as a decode that returns whatever was encoded, test it by property-based testing: state the law as a property and check it over generated inputs rather than at chosen examples.
Chosen examples reach only the inputs their author thought of, while a law stays true across a rewrite and generated inputs search for the one that breaks it.

- Have the property assert the law at the strength it was stated, since a weaker consequence of it passes against code that breaks the law.
- Constrain the result without recomputing it the way the code does, since an expected value computed the same way shares the code's bugs.
- Where an edge of the domain is already known, pin it as an example beside the property, since generated inputs may not reach it.
- Where the project has no library that generates inputs, do not add one unasked, a dependency being the user's to accept: generate the inputs exhaustively with what the language has, over a bounded domain drawn from the whole input type and not only from the values the law names, and report that a library would widen it.

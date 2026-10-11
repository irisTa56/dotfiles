---
name: test-drive
description: "Build or change behavior by test-driven development as Kent Beck describes it: list the behaviors, then, one behavior at a time, write its test, watch it fail, and write the code that passes it. Use whenever implementing a feature, changing what existing code does, or fixing a bug in a codebase that has automated tests or should have them, including when the request says nothing about tests."
---

# Test Drive

Build the change by test-driven development as Kent Beck describes it in [Canon TDD](https://newsletter.kentbeck.com/p/canon-tdd): write each test just before the code it asks for, and write no production code a failing test has not asked for.

## The loop

1. List the behaviors the change has to have, a behavior being one thing a caller of the code can observe.
   - The list is a plan and not yet tests, and it grows as the work turns up cases.
2. Turn exactly one item into a test, or, where the work changes what existing code does, rewrite the tests that pin the old behavior so that they state the new one.
3. Run it and watch it fail, and read the failure: it should be the behavior that is missing.
   - A test that passes at once has pinned nothing the code lacked, so find out why before going on.
4. Write the least code that makes it and every earlier test pass.
5. Restructure only while the suite is green, then go back to step 2 until the list is empty.

Do not turn several items into tests ahead of their code.
Tests written in a batch describe the design as imagined before any of it ran, and tests written after the code describe what the code happens to do.

## Where a test attaches

- Where a spec or a plan has settled what is verified and at which boundary, follow it.
- Where none has, take both from the user's instruction: the behavior it describes, at the boundary its caller reaches it through.
- Ask only where the instruction leaves open something that changes what gets built, and do not stop partway through to confirm what it settles.
- Test a module through what it offers its callers, and give no test of its own to a function the module keeps private, such as a `defp` in Elixir.
  - The public function that uses it is where its behavior is tested, which keeps the tests few and lets the inside be restructured without rewriting them.

## A law over a whole input domain

Where the behavior is a law that holds over a whole input domain, such as a decode that returns whatever was encoded, write its test as a property over generated inputs rather than as examples at chosen ones.
Have the property assert the law at the strength it was stated, not a weaker consequence of it, and constrain the result without recomputing it the way the code does.
Where the project has no library that generates inputs, do not add one unasked: generate a small input domain exhaustively with what the language has, and report that a library would widen it.

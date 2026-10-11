---
name: test-first
description: "Build or change behavior test first: write the test for one behavior, watch it fail, write the code that passes it, and repeat for the next behavior. Use whenever implementing a feature, changing what existing code does, or fixing a bug in a codebase that has automated tests or should have them, including when the request says nothing about tests."
---

# Test First

Write the test for a behavior before the code that gives it, and write no production code a failing test has not asked for.

## The loop

Take one behavior at a time, a behavior being one thing a caller of the code can observe.

1. Write the test for it, or, where the work changes what existing code does, rewrite the tests that pin the old behavior so that they state the new one.
2. Run it and watch it fail, and read the failure: it should be the behavior that is missing.
   - A test that passes at once has pinned nothing the code lacked, so find out why before going on.
3. Write the least code that makes it pass, then run the whole suite.
4. Restructure only while the suite is green, and then take the next behavior.

Do not write the tests for several behaviors ahead of their code.
Tests written in a batch describe the design as imagined before any of it ran, and tests written after the code describe what the code happens to do.

## Where a test attaches

- Where a spec or a plan has settled what is verified and at which boundary, follow it.
- Where none has, take both from the user's instruction: the behavior it describes, at the boundary its caller reaches it through.
- Ask only where the instruction leaves open something that changes what gets built, and do not stop partway through to confirm what it settles.

## A law over a whole input domain

Where the behavior is a law that holds over a whole input domain, such as a decode that returns whatever was encoded, write its test as a property over generated inputs rather than as examples at chosen ones.
Have the property assert the law at the strength it was stated, not a weaker consequence of it, and constrain the result without recomputing it the way the code does.

---
description: What this user adds to the rust-best-practices skill, and where they depart from it, on modules, crates, methods, traits and dependencies
paths:
  - "**/*.rs"
---

# Rust

Where the `rust-best-practices` skill is on, follow it with the additions and exceptions below; where a project leaves it off, these rules apply alone.
Where the code around an edit already follows a convention one of them differs from, stay consistent with that code instead.

## Modules and crates

- The skill keeps a non-leaf module to `mod` declarations and re-exports. Depart from that where a module grows too large to read as one, or where several implementations of a behaviour share one data structure: the parent module then defines the struct or trait, and its child modules provide the processing, as `impl` blocks and trait implementations.
  - A child then depends on its parent only for the abstraction, and the parent names nothing a child defines.
  - A child reads its parent's private fields, which Rust lets a descendant module do; any other module builds and reads the value through the type's methods.
  - A child does not call a private function of its parent: such a function is processing the child needs, kept beside the data rather than apart from it, so the parent has to know what the child does.
- A project with a single crate may keep it as one package in a subdirectory rather than in a `crates/` workspace, as the one-crate form of the skill's flat workspace.
- A crate you create sets the [`unreachable_pub`](https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unreachable-pub) lint, off by default, to `warn`, so an item marked `pub` that nothing outside the crate reaches is flagged; the lint's own fix is `pub(crate)`.

## Methods and functions

- Write processing as a method of the type that holds what it reads, and where several types do, of the most specific one.
  - A general type then needs no knowledge of a specific one.
- The skill makes any computation with no receiver a free function; here it is one in these two cases and in no other:
  - what it reads is spread over types that are peers, none of them holding another
  - the type that fits belongs to another crate
- In every other case define a type for it, holding no more than the processing reads.
- A function that is not private takes no `&mut` parameter other than `&mut self`, unless a trait it implements fixes the signature.
  - What has to change a value is a method of that value's type where the crate defines the type, and otherwise returns the new value.
  - Only a private function has all its callers in the module, where which of them owns the value can be read.

## Traits

- Define a trait from what the code using it needs, not from what the implementing side has, for example:
  - a type whose contents may be replaced while its behaviour stays
  - a parameter several functions take that should be one thing, such as a closure given a name
- A trait wanted only as a return type has come from the implementing side, so question it before adding it.

## Dependencies

- Where the standard library, a crate the code already depends on, or a widely used, maintained crate with few dependencies of its own does a job and takes code away, use it rather than writing the job out by hand: weigh the costs the skill names for a dependency against that reduction, rather than avoiding one by default.

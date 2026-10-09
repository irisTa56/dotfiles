---
description: How to lay out Rust modules, methods, traits and visibility, and when to take a dependency
paths:
  - "**/*.rs"
---

# Rust

Follow these rules when writing Rust.
Where the code around an edit already follows a convention one of them differs from, stay consistent with that code instead.

## Modules

- Build a module around a struct or a trait: the parent module defines it, and its child modules provide the processing, as `impl` blocks and trait implementations.
  - A child then depends on its parent only for the abstraction, and the parent names nothing a child defines.
- A field is private. A child module reads its parent's private fields, which Rust lets a descendant module do; any other module builds and reads the value through the type's methods.
- A child module does not call a private function of its parent.
  - Such a function is processing the child needs, kept beside the data rather than apart from it, so the parent has to know what the child does.
- An item is private unless something outside its module uses it, and `pub` only where something outside the crate does.
  - The [`unreachable_pub`](https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unreachable-pub) lint checks the second where it is on, and it is off by default: a crate you create sets it to `warn`. Nothing checks the first, and the lint's own fix is `pub(crate)`.

## Methods and functions

- Write processing as a method of the type that holds what it reads, and where several types do, of the most specific one.
  - A general type then needs no knowledge of a specific one.
- Where no type of the crate fits, the processing is a free function in these two cases and in no other:
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

- Where the standard library, a crate the code already depends on, or a widely used crate with few dependencies of its own does a job, use it rather than writing the job out by hand.

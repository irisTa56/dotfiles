---
description: How to lay out Rust modules, crates, methods, traits and visibility, and when to take a dependency
paths:
  - "**/*.rs"
  - "**/Cargo.toml"
---

# Rust

Follow these rules when writing Rust.
Where the `rust-best-practices` skill is on and says otherwise, these rules take precedence.
Where the code around an edit already follows a convention one of them differs from, stay consistent with that code instead.

## Modules

- Build a module around a struct or a trait, in one file.
- Split it into a parent module and child modules where there is a reason to, such as the module growing too large to read as one, or several implementations of a behaviour sharing one data structure.
  - The parent keeps the definition of the struct or trait, not only the declarations of its children, and the children provide the processing, as `impl` blocks and trait implementations.
  - A child then depends on its parent only for the abstraction, and the parent names nothing a child defines.
  - A child module does not call a private function of its parent: such a function is processing the child needs, kept beside the data rather than apart from it, so the parent has to know what the child does.
- A field is private, in a struct that only carries data as well.
  - A child module reads its parent's private fields, which Rust lets a descendant module do; any other module builds and reads the value through the type's methods.
- An item is private unless something outside its module uses it, and `pub` only where something outside the crate does.
  - The [`unreachable_pub`](https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html#unreachable-pub) lint checks the second where it is on, and it is off by default: a crate you create sets it to `warn`. Nothing checks the first, and the lint's own fix is `pub(crate)`.

## Crates

- A module split into child modules, with the directory that takes, is no reason by itself to split its crate.
- A project with a single crate may keep it as one package in a subdirectory of the repository, rather than in a workspace.

## Methods and functions

- Write processing as a method of the type that holds what it reads, and where several types do, of the most specific one.
  - A general type then needs no knowledge of a specific one.
- Where no type of the crate fits, the processing is a free function in these two cases and in no other:
  - what it reads is spread over types that are peers, none of them holding another
  - the type that fits belongs to another crate
- In every other case, including a computation that has no receiver, define a type for it, holding no more than the processing reads.
- A function that is not private takes no `&mut` parameter other than `&mut self`, whether the value is of a type the crate defines, of another crate's type, or a buffer the caller reuses, unless a trait it implements fixes the signature.
  - What has to change a value is a method of that value's type where the crate defines the type, and otherwise returns the new value.
  - Only a private function has all its callers in the module, where which of them owns the value can be read.
- Where a function runs several stages one after another, give each stage a function of its own, so that the function reads as its stages called in order.
  - This holds only where the detail of the stages hides their order from the function's reader.
  - Stages are, for example, checking the input, reading what the function acts on and writing the result.

## Traits

- Define a trait from what the code using it needs, not from what the implementing side has, for example:
  - a type whose contents may be replaced while its behaviour stays
  - a parameter several functions take that should be one thing, such as a closure given a name
- A trait wanted only as a return type has come from the implementing side, so question it before adding it.

## Dependencies

- Where the standard library, a crate the code already depends on, or a widely used crate with few dependencies of its own does a job, use it rather than writing the job out by hand.

# nabi-macros

Procedural macros for Nabi: `#[main]`, `#[conductor]`, `#[quantum]`, `#[flat]`, `select!`, `join!`.

Part of the [Nabi async runtime](https://nabi.run).

This crate provides the attribute and function-like macros that back Nabi's ergonomics. `#[nabi::conductor]` generates dual-scheduler code paths with phantom-type tagging to prevent cross-mode value escape at compile time. `#[nabi::flat]` validates `repr(C)` layout for distributed zero-copy messaging. `select!` and `join!` are familiar async combinators adapted to Nabi's runtime. The macros are re-exported from the top-level [`nabi`](https://docs.rs/nabi) facade; direct use of this crate is generally unnecessary.

## Overview

- `main/` — `#[nabi::main]` runtime entry point
- `test/` — `#[nabi::test]` for async tests
- `conductor/` — `#[nabi::conductor]` with dual scheduler code generation, phantom tagging, edge parsing, IR emission
- `stage/` — `#[nabi::stage]` stage definition
- `quantum/` — `#[nabi::quantum]` batch unit
- `flat/` — `#[nabi::flat]` FlatLayout attribute with layout verification
- `select/` — `select!` combinator
- `join/` — `join!` and `try_join!` combinators
- `pin/` — `pin!` helper

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

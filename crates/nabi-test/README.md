# nabi-test

Test utilities for Nabi: mocks, assertions, fixtures.

Part of the [Nabi async runtime](https://nabi.run).

This crate provides test helpers for crates in the Nabi workspace and for downstream users building on Nabi. Mocks implement Nabi's core traits (`Axon`, `Clock`, `Runtime`, `Conductor`) with controllable behavior. Assertion helpers inspect arena state, task lifecycle, and completion events. Fixtures spin up isolated TCP listeners, temporary files, and minimal Conductors for integration tests. Like [`tokio-test`](https://docs.rs/tokio-test), this crate has zero external dependencies — testing techniques such as property-based testing (`proptest`), loom permutation testing, or temp-file management are added as dev-deps on a per-crate basis.

## Overview

- `mock/` — `MockAxon`, `MockRuntime`, `MockClock`, `MockConductor`
- `context.rs` / `runtime.rs` — lightweight test context and runtime harness
- `assertion/` — inspection helpers for tasks, arenas, and completions
- `fixture/` — ready-made TCP listener, temporary file, and Conductor fixtures

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

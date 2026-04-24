# nabi-core

Shared primitives for Nabi: `NabiId`, errors, scheduling hints, cancellation kinds, namespace IDs, and the `FlatLayout` trait for zero-copy distributed messaging.

Part of the [Nabi async runtime](https://nabi.run).

This crate is the leaf of the Nabi workspace — it has no external dependencies and is consumed by every other `nabi-*` crate. Direct use from application code is generally unnecessary; the top-level [`nabi`](https://docs.rs/nabi) facade re-exports everything needed.

## Overview

- `NabiId` — 64-bit identifier with embedded worker ID, generation, and slot index
- `NabiError` — unified error type with zero external dependencies
- `SchedulingHint` / `AffinityHint` — scheduler and CPU affinity hints
- `CancellationKind` — explicit cancellation variants
- `NamespaceId` — interned namespace identifier for horizontal tracing
- `FlatLayout` — trait for wire-format-compatible types (distributed zero-copy)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

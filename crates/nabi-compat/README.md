# nabi-compat

Tokio compatibility layer for Nabi: API shim for axum, tonic, sqlx, hyper.

Part of the [Nabi async runtime](https://nabi.run).

This crate mirrors Tokio's public API surface on top of Nabi's runtime, enabling ecosystem libraries written against `tokio::*` to run on Nabi without modification. The `AsyncRead` / `AsyncWrite` traits, channel types, timer primitives, and `JoinHandle` semantics are all reproduced. Internally, requests are translated through the `emulator` module into Nabi's completion-based I/O and typed index-handle ownership; the cost is readiness-to-completion bridging on the hot path, making `nabi-compat` a migration tool rather than a performance target. Native Nabi APIs in `nabi-io`, `nabi-runtime`, and `nabi-orchestration` remain the preferred path for new code.

## Overview

- `runtime/` — `tokio::runtime::{Handle, Builder}`, `spawn`, `block_on` equivalents
- `io/` — `AsyncRead`, `AsyncWrite`, `AsyncBufRead`, `AsyncBufWrite`, `AsyncSeek`, poll bridging
- `net/` — `TcpStream`, `TcpListener`, `UdpSocket`, Unix variants
- `fs/` — `File`, path operations
- `time/` — `sleep`, `interval`, `timeout`, `Instant`
- `sync/` — `Mutex`, `RwLock`, `Semaphore`, `Notify`, `oneshot`, `mpsc`, `broadcast`, `watch`
- `task/` — `JoinHandle`, `spawn_blocking`, `yield_now`
- `macros/` — aliases for `select!` and friends
- `emulator/` — readiness/waker/buffer/cancel adapters (internal)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

# nabi-io

Platform I/O for Nabi: io_uring (Linux), IOCP (Windows), epoll (Linux fallback), kqueue (macOS/BSD).

Part of the [Nabi async runtime](https://nabi.run).

This crate provides the `Axon` trait — Nabi's unified async I/O abstraction — and its platform-specific implementations. The trait exposes completion-based operations, registered buffers, multishot capabilities, and cross-ring messaging where supported; epoll and kqueue receive degraded default implementations for parity.

## Overview

- `Axon` — unified I/O trait with full kernel capability surface
- `uring/` — Linux io_uring backend with `DEFER_TASKRUN`, `SINGLE_ISSUER`, `buf_ring`, `msg_ring`, registered buffers
- `iocp/` — Windows IOCP backend (IoRing reserved for future 22H2+ support)
- `epoll/` — Linux fallback for older kernels
- `kqueue/` — macOS/BSD backend (thin, development and test use)
- `buffer/` — buffer pool, ring buffer, registered buffer, vectored I/O
- `operation/` — operation codes, tokens, completions, request types
- `capability/` — runtime kernel feature detection

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

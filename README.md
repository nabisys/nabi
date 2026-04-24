# nabi

Async runtime for Rust: io_uring, dual scheduler, orchestration.

[Website](https://nabi.run) · [Documentation](https://docs.rs/nabi) · [GitHub](https://github.com/nabisys/nabi)

## What Nabi is

Nabi is an async runtime built around three ideas:

1. **Completion-first I/O.** Linux io_uring is the primary path, with IOCP on Windows and epoll/kqueue as degraded fallbacks. Registered buffers, `buf_ring`, `msg_ring`, and multishot are exposed as first-class capabilities.
2. **Dual scheduler with compile-time separation.** Two scheduler modes — affine (thread-per-core, `!Send` tasks) and stealing (work-stealing, `Send` tasks) — coexist. Phantom-type tagging prevents cross-mode value escape at compile time.
3. **Orchestration integrated into the runtime.** The `Conductor` owns an arena and a typed DAG of `Stage` nodes. Per-stage scheduler selection, explicit cancellation semantics, and resilience primitives (`Advisor::guard()` composition) are runtime features, not middleware bolted on top.

## Quick start

```toml
[dependencies]
nabi = "0.1"
```

```rust
#[nabi::main(scheduler = "stealing")]
async fn main() -> nabi::Result<()> {
    let listener = nabi::net::TcpListener::bind("0.0.0.0:8080").await?;
    loop {
        let (stream, _) = listener.accept().await?;
        nabi::runtime::run_stealing(handle(stream));
    }
}

async fn handle(stream: nabi::net::TcpStream) { /* ... */ }
```

## Features

| feature | on by default | includes |
|---|---|---|
| `macros` | ✓ | `#[nabi::main]`, `#[nabi::conductor]`, `#[nabi::quantum]`, `#[nabi::flat]`, `select!`, `join!` |
| `net` | | TCP, UDP, Unix sockets |
| `fs` | | files, directories, pipes, splice/tee |
| `tls` | | rustls and native-tls backends (implies `net`) |
| `compat` | | Tokio API shim for axum/tonic/sqlx/hyper |
| `lens` | | observability emission |
| `full` | | everything above |

Core runtime (`nabi-core`, `nabi-io`, `nabi-runtime`, `nabi-orchestration`) is always included.

## Ecosystem

- `nabi-scope` — observability TUI (`cargo install nabi-scope`)
- `nabi-test` — test utilities (dev-dep only)

## Documentation

- Tutorials: <https://nabi.run/learn>
- How-to guides: <https://nabi.run/guide>
- Architecture: <https://nabi.run/architecture>
- API reference: <https://docs.rs/nabi>

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.

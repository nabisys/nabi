# nabi-scope

Observability TUI for Nabi: live inspection of tasks, conductors, axons.

Part of the [Nabi async runtime](https://nabi.run).

This crate is a `cargo install`-able TUI binary plus a reusable client library. It reads events emitted by [`nabi-lens`](https://docs.rs/nabi-lens) over a Unix domain socket (or ring-buffer transport), reconstructs runtime state, and presents it interactively. Pages include task trees, conductor DAG execution, axon activity, advisor decisions, and flame graphs. The CLI subcommands support one-shot snapshots, streaming, assertions (for integration tests), and exporting to Prometheus, OpenTelemetry, Elastic, or file sinks.

## Usage

```bash
cargo install nabi-scope
nabi-scope                                      # interactive TUI
nabi-scope snapshot --output state.json         # one-shot
nabi-scope export --format prometheus --port 9090
```

## Overview

- `client/` — transport-agnostic client with reconnection (exposed as library)
- `model/` — runtime state model (task tree, conductor, axon, arena, metrics, event buffer)
- `query/` — query builders by NID, namespace, conductor, time range, aggregation
- `tui/` — interactive terminal UI (overview, task tree, conductor, axon, advisor, flamegraph, events pages)
- `cli/` — subcommands (snapshot, stream, assert, inspect, diff)
- `export/` — Prometheus, Elastic, OTLP, file exporters
- `remote/` — remote connection with authentication

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

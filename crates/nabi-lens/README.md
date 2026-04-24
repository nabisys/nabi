# nabi-lens

Observability emission for Nabi: events, sinks, sampling, metrics.

Part of the [Nabi async runtime](https://nabi.run).

This crate is Nabi's observability emission layer. It defines the wire format for runtime events (task lifecycle, conductor execution, axon operations, arena allocation, scheduler decisions, advisor outcomes), provides sinks (Unix domain socket, stdout, ring buffer, null), and exposes sampling strategies. Downstream consumers — most notably [`nabi-scope`](https://docs.rs/nabi-scope) — read these events over the wire. The crate uses zero external dependencies; events are encoded in a self-describing binary format defined in `wire/`.

## Overview

- `event/` — task, conductor, arena, axon, scheduler, advisor events with common fields
- `field/` — reusable field types (NabiId, Namespace, Duration, Value)
- `layer/` — subscriber integration with filter and context
- `sink/` — sink trait, Unix socket, stdout, ring buffer, null implementations
- `wire/` — binary wire format, encode/decode, version negotiation
- `sampling/` — always, ratio, adaptive, per-conductor sampling strategies
- `metric/` — counter, histogram, gauge, registry
- `snapshot/` — runtime, task tree, arena, axon state snapshots

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

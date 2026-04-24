# nabi-orchestration

Orchestration for Nabi: Conductor DAGs, stages, quantum batching, resilience.

Part of the [Nabi async runtime](https://nabi.run).

This crate provides Nabi's declarative execution layer. A `Conductor` owns an arena and a directed acyclic graph of `Stage` nodes; data flows along typed edges, with `@node` annotations distinguishing local from remote transport. Per-stage scheduler mode selection (Affine or Stealing) is compile-time enforced via phantom-type tagging of stage I/O. Resilience is expressed as explicit composition — `Advisor::guard()` layers acquire, timeout, retry, and circuit breaking in a fixed order — rather than ambient middleware.

## Overview

- `conductor/` — the Conductor itself: builder, planner, execution plan, graph, lifecycle
- `stage/` — `Stage` trait, multi-input variants, code generation
- `chain/` — linear pipelines, recovery branches
- `quantum/` — micro-batching, concurrency control, backpressure, failure policy
- `advisor/` — `guard()` composition of acquire/timeout/retry/breaker/execute
- `retry/` — retry policies, backoff strategies, jitter, error classification
- `breaker/` — circuit breaker with open/half-open/closed states
- `limiter/` — token-bucket rate limiter
- `cancellation/` — tree-based propagation, intrusive child lists
- `context/` — per-request metadata with TypeId-indexed arena slots
- `ir/` — `NabiIR`, `ConductorSpec`, `PolicyKind`, `ServiceRegistry`, `IntoRuntime`

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

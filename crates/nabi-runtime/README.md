# nabi-runtime

Runtime core for Nabi: scheduler, workers, tasks, timers, sync primitives.

Part of the [Nabi async runtime](https://nabi.run).

This crate provides Nabi's execution engine. Two scheduler variants coexist: work-stealing for multi-threaded Send workloads, and thread-per-core for !Send workloads with strict CPU affinity. Tasks are stored in per-worker sharded slabs with generational indices; cross-worker reclaim goes through an MPSC queue. The timer wheel, blocking pool, signal handling, and process management sit alongside the scheduler as first-class runtime services.

## Overview

- `scheduler/` — work-stealing and thread-per-core variants, dispatch, queue, park, priority
- `worker/` — sharded task slab, main loop, reclaim, wake
- `task/` — `TaskHeader`, `TaskRef` bit layout, atomic state, `IndexWaker`, children list
- `memory/` — `Arena` (bump allocator, generation, phase) and `Slab` (generational key)
- `timer/` — hashed timing wheel, slot, entry, clock
- `sync/` — `Mutex`, `RwLock`, `Semaphore`, `Notify`, `Barrier`, `Once`, channels (mpsc/oneshot/broadcast/watch)
- `time/` — `sleep`, `interval`, `Instant`
- `signal/`, `process/`, `blocking/` — OS integration and blocking task pool
- `runtime/` — `run_affine`, `run_stealing`, `run_blocking` entry points and builder

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

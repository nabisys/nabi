//! `nabi-runtime` `memory/` — runtime memory layer.
//!
//! Two structures back the runtime:
//!
//! * [`slab`] — per-worker generational slab for spawn-path tasks.
//! * `arena` — Conductor DAG bump allocator (follow-up PR).

pub mod slab;

//! Runtime core for Nabi: scheduler, worker, task, memory, timer, async sync primitives.
//!
//! Defines the per-process machinery on which orchestration and I/O run:
//!
//! - [`memory`] — per-worker generational slab for spawn-path tasks
//!
//! No I/O backends — those live in `nabi-io`.
#![allow(
    unused_crate_dependencies,
    reason = "deps wired ahead of further runtime modules"
)]

pub mod memory;

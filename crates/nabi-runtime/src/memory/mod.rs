//! `nabi-runtime` `memory/` — runtime memory layer.
//!
//! Two storage backends sharing one generational counter:
//!
//! - [`slab`] — per-worker, fixed-capacity slab for spawn-path tasks.
//! - [`arena`] — Conductor DAG bump allocator with bulk reset.
//!
//! Both consume [`Generation`] from the [`generation`] submodule.

pub mod arena;
pub mod generation;
pub mod slab;

pub use generation::Generation;

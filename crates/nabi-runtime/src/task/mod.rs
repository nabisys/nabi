//! `nabi-runtime` `task/` — typed task handles, mode markers, lifecycle
//! state, the per-task control block, and intrusive children helpers.
//!
//! - [`TaskRef`] — 64-bit packed handle, path-agnostic over slab and arena.
//! - [`Affine`] / [`Stealing`] — phantom markers selected by the [`Mode`] trait.
//! - `AtomicTaskState` — atomic CAS-based lifecycle (crate-internal).
//! - `TaskHeader` — repr(C) control block (crate-internal).
//! - `children` — intrusive child list helpers over a `TaskStorage`.

mod children;
mod header;
mod marker;
mod state;
mod task_ref;

pub use marker::{Affine, Mode, Stealing};
#[allow(unused_imports, reason = "no non-test caller in this revision")]
pub(crate) use state::{AtomicTaskState, TaskState};
pub use task_ref::TaskRef;

//! `nabi-runtime` `task/` — typed task handles, mode markers, lifecycle
//! state, the per-task control block, intrusive children helpers, and the
//! type-erased waker.
//!
//! - [`TaskRef`] — 64-bit packed handle, path-agnostic over slab and arena.
//! - [`Affine`] / [`Stealing`] — phantom markers selected by the [`Mode`] trait.
//! - `AtomicTaskState` — atomic CAS-based lifecycle (crate-internal).
//! - `TaskHeader` / `Slot<F>` / `Cell<F>` / `TaskVTable` — repr(C) per-task
//!   control block + cell layout (crate-internal).
//! - `children` — intrusive child list helpers over a `TaskStorage`.
//! - `waker` — `RawWakerVTable` and the `TaskRef → Waker` adapter.

mod children;
mod header;
mod marker;
mod state;
mod task_ref;
mod waker;

pub use marker::{Affine, Mode, Stealing};
#[allow(unused_imports, reason = "no non-test caller in this revision")]
pub(crate) use state::{AtomicTaskState, TaskState};
pub use task_ref::TaskRef;

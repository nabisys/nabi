//! `nabi-runtime` `task/` — typed task handles, mode markers, and lifecycle
//! state.
//!
//! Surfaces grow incrementally per the M1 P2 plan:
//!
//! - [`TaskRef`] — 64-bit packed handle, path-agnostic over slab and arena.
//! - [`Affine`] / [`Stealing`] — phantom markers selected by the [`Mode`] trait.
//! - `AtomicTaskState` — atomic lifecycle (crate-internal; consumed by the
//!   upcoming header + waker layer).
//!
//! Subsequent PRs add the intrusive children list, the `Header + Cell` layout,
//! the `IndexWaker` vtable, and the public `TaskHandle<T, Mode>` surface.

mod marker;
mod state;
mod task_ref;

pub use marker::{Affine, Mode, Stealing};
#[allow(
    unused_imports,
    reason = "consumed by upcoming P2 PR4 task::header and task::waker"
)]
pub(crate) use state::{AtomicTaskState, TaskState};
pub use task_ref::TaskRef;

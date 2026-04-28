//! `nabi-runtime` `task/` — typed task handles and mode markers.
//!
//! P2 PR1 surface — leaf modules with no dependency on later PRs:
//!
//! - [`TaskRef`] — 64-bit packed handle, path-agnostic over slab and arena.
//! - [`Affine`] / [`Stealing`] — phantom markers selected by the [`Mode`] trait.
//!
//! Subsequent PRs add the atomic state machine, the `Header + Cell` layout, the
//! intrusive children list, the `IndexWaker` vtable, and the public
//! `TaskHandle<T, Mode>` surface.

mod marker;
mod task_ref;

pub use marker::{Affine, Mode, Stealing};
pub use task_ref::TaskRef;

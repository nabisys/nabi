//! Foundational types shared by all Nabi crates.
//!
//! Defines the leaf vocabulary on which the rest of the workspace builds:
//!
//! - [`Nid`] — 128-bit tree-structured observability identifier
//! - [`NidError`] — errors from [`Nid`] operations
//! - [`AffinityHint`], [`SchedulingHint`] — task placement and scheduler hints
//! - [`FlatLayout`] — marker trait for types with stable byte layout
//!
//! Future modules: `cancellation`, `namespace`.

pub mod flat;
pub mod hint;
pub mod id;

pub use flat::FlatLayout;
pub use hint::{AffinityHint, SchedulingHint};
pub use id::{Nid, NidError};

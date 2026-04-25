//! Foundational types shared by all Nabi crates.
//!
//! Defines the leaf vocabulary on which the rest of the workspace builds:
//!
//! - [`Nid`] — 128-bit tree-structured observability identifier
//! - [`NidError`] — errors from [`Nid`] operations
//!
//! Future modules: `hint`, `cancellation`, `namespace`, `flat`.

pub mod id;

pub use id::{Nid, NidError};

//! `nabi-core` `id/` — [`Nid`] tree-structured observability primitive.

mod display;
mod error;
mod generate;
mod layout;
mod nid;
mod relation;

pub use error::NidError;
pub use nid::Nid;

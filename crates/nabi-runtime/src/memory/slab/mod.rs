//! `nabi-runtime` `memory/slab/` — per-worker generational slab.

mod key;
#[allow(
    clippy::module_inception,
    reason = "intentional: `Slab<T>` impl colocated with the `slab` module"
)]
mod slab;

pub use key::SlabKey;
pub use slab::{Slab, SlabError};

//! Linux `io_uring` backend.
//!
//! Minimal ring bootstrap, kernel feature detection, and `IORING_OP_NOP`
//! round-trip. The full `Axon` trait, buffer registration, multishot, and
//! `msg_ring` facilities land alongside the full axon layer.

pub mod axon;
pub mod detect;

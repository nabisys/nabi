//! Linux `io_uring` backend.
//!
//! Kernel feature detection and a minimal axon implementation used for
//! ring bootstrap and verifying SQ/CQ wiring. The full `Axon` trait,
//! buffer registration, multishot, and `msg_ring` facilities land
//! alongside the full axon layer.

pub mod detect;

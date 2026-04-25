//! Bit layout constants for [`super::Nid`].
//!
//! ```text
//! [127..80] seq      — 48 bits, monotonic sequence number
//! [ 79..64] depth    — 16 bits, parent-child nesting depth
//! [ 63..48] kind     — 16 bits, reserved for post-P0 (task/conductor/stage tag)
//! [ 47..40] version  —  8 bits, layout version
//! [ 39.. 2] worker   — 38 bits, originating worker / routing hint
//! [  1.. 0] flags    —  2 bits, reserved for post-P0
//! ```

pub(super) const SEQ_SHIFT: u32 = 80;
pub(super) const SEQ_BITS: u32 = 48;
pub(super) const SEQ_MASK: u128 = ((1u128 << SEQ_BITS) - 1) << SEQ_SHIFT;

pub(super) const DEPTH_SHIFT: u32 = 64;
pub(super) const DEPTH_BITS: u32 = 16;
pub(super) const DEPTH_MASK: u128 = ((1u128 << DEPTH_BITS) - 1) << DEPTH_SHIFT;

pub(super) const VERSION_SHIFT: u32 = 40;

pub(super) const WORKER_SHIFT: u32 = 2;
pub(super) const WORKER_BITS: u32 = 38;
pub(super) const WORKER_MASK: u128 = ((1u128 << WORKER_BITS) - 1) << WORKER_SHIFT;

/// Current layout version stored in every `Nid`.
pub(super) const CURRENT_VERSION: u8 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    /// Layout invariant: bit field sizes must sum to 128.
    #[test]
    fn total_bits_sum_to_128() {
        const KIND_BITS: u32 = 16;
        const VERSION_BITS: u32 = 8;
        const FLAGS_BITS: u32 = 2;
        let total = SEQ_BITS + DEPTH_BITS + KIND_BITS + VERSION_BITS + WORKER_BITS + FLAGS_BITS;
        assert_eq!(total, 128);
    }

    /// Active masks (those used by P0 accessors) must not overlap.
    #[test]
    fn p0_masks_non_overlapping() {
        let or = SEQ_MASK | DEPTH_MASK | WORKER_MASK;
        let xor = SEQ_MASK ^ DEPTH_MASK ^ WORKER_MASK;
        assert_eq!(or, xor);
    }
}

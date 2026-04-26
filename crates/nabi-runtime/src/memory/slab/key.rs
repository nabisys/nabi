//! [`SlabKey`] handle and [`Generation`] counter.

use core::fmt;

const INDEX_BITS: u32 = 32;
const GENERATION_BITS: u32 = 24;
const INDEX_MASK: u64 = (1u64 << INDEX_BITS) - 1;
const GENERATION_MASK: u64 = (1u64 << GENERATION_BITS) - 1;
const GENERATION_SHIFT: u32 = INDEX_BITS;

/// 24-bit wrapping generation counter for ABA prevention.
///
/// Slot occupancy is encoded by parity:
///
/// * **odd** — slot is occupied
/// * **even** — slot is empty
///
/// Wrapping at `2^24` is acceptable: a slot must be recycled `2^24` times
/// before a stale [`SlabKey`] could collide.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct Generation(pub(crate) u32);

impl Generation {
    /// Empty initial state — even, never inserted into.
    pub const ZERO: Self = Self(0);

    /// Maximum 24-bit value (`2^24 - 1`).
    pub const MAX: u32 = (1 << GENERATION_BITS) - 1;

    /// Returns the raw value masked to 24 bits.
    #[inline]
    pub const fn get(self) -> u32 {
        self.0 & Self::MAX
    }

    /// Returns `true` when the generation parity marks a slot as occupied.
    #[inline]
    pub const fn is_occupied(self) -> bool {
        self.0 & 1 == 1
    }

    /// Increments the generation, wrapping at `2^24`.
    #[inline]
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.wrapping_add(1) & Self::MAX)
    }
}

impl fmt::Debug for Generation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Generation({})", self.get())
    }
}

/// 64-bit handle returned by [`Slab::insert`].
///
/// Bit layout:
///
/// ```text
/// [63..56] reserved   — 8 bits, zero (TaskRef tag/routing overlay)
/// [55..32] generation — 24 bits, insert-time generation
/// [31.. 0] index      — 32 bits, slot index
/// ```
///
/// `Copy`. The reserved high bits are guaranteed zero so `TaskRef` may
/// overlay metadata on them in P2.
///
/// [`Slab::insert`]: super::Slab::insert
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SlabKey(u64);

impl SlabKey {
    #[inline]
    pub(super) const fn new(index: u32, generation: Generation) -> Self {
        let bits = ((generation.get() as u64) << GENERATION_SHIFT) | index as u64;
        Self(bits)
    }

    /// Returns the slot index.
    #[inline]
    pub const fn index(self) -> u32 {
        (self.0 & INDEX_MASK) as u32
    }

    /// Returns the generation captured at insert time.
    #[inline]
    pub const fn generation(self) -> Generation {
        Generation(((self.0 >> GENERATION_SHIFT) & GENERATION_MASK) as u32)
    }

    /// Returns the raw 64-bit representation.
    #[inline]
    pub const fn to_bits(self) -> u64 {
        self.0
    }
}

impl fmt::Debug for SlabKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlabKey")
            .field("index", &self.index())
            .field("generation", &self.generation().get())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_zero_is_even_and_empty() {
        assert_eq!(Generation::ZERO.get(), 0);
        assert!(!Generation::ZERO.is_occupied());
    }

    #[test]
    fn generation_next_alternates_parity() {
        let g0 = Generation::ZERO;
        let g1 = g0.next();
        let g2 = g1.next();
        assert!(g1.is_occupied());
        assert!(!g2.is_occupied());
        assert_eq!(g1.get(), 1);
        assert_eq!(g2.get(), 2);
    }

    #[test]
    fn generation_wraps_at_24_bit_max() {
        let max = Generation(Generation::MAX);
        assert_eq!(max.get(), Generation::MAX);
        assert!(max.is_occupied());
        assert_eq!(max.next().get(), 0);
    }

    #[test]
    fn generation_pre_max_boundary() {
        let pre_max = Generation(Generation::MAX - 1);
        assert_eq!(pre_max.next().get(), Generation::MAX);
    }

    #[test]
    fn slabkey_layout_index_generation_split() {
        let key = SlabKey::new(0xDEAD_BEEF, Generation(0x123_456));
        assert_eq!(key.index(), 0xDEAD_BEEF);
        assert_eq!(key.generation().get(), 0x123_456);
    }

    #[test]
    fn slabkey_reserved_bits_are_zero() {
        let key = SlabKey::new(u32::MAX, Generation(Generation::MAX));
        assert_eq!(key.to_bits() >> 56, 0);
    }

    #[test]
    fn slabkey_repr_size() {
        assert_eq!(core::mem::size_of::<SlabKey>(), core::mem::size_of::<u64>());
        assert_eq!(
            core::mem::size_of::<Generation>(),
            core::mem::size_of::<u32>()
        );
    }
}

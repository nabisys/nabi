//! [`BumpAllocatorBuilder`] — fluent constructor for [`BumpAllocator`].
//!
//! [`BumpAllocator`]: super::BumpAllocator

use super::bump::{ArenaError, BumpAllocator};

/// Default backing capacity in bytes when `bytes` is not set.
pub const DEFAULT_BYTES: usize = 64 * 1024;

/// Default drop-slot capacity when `drop_slots` is not set. Zero
/// forbids `alloc_with_drop` entirely.
pub const DEFAULT_DROP_SLOTS: usize = 0;

/// Builder for [`BumpAllocator`]. Obtain via [`BumpAllocator::builder`].
///
/// [`BumpAllocator`]: super::BumpAllocator
/// [`BumpAllocator::builder`]: super::BumpAllocator::builder
#[derive(Clone, Copy, Debug)]
pub struct BumpAllocatorBuilder {
    bytes: usize,
    drop_slots: usize,
}

impl Default for BumpAllocatorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BumpAllocatorBuilder {
    #[inline]
    pub(super) const fn new() -> Self {
        Self {
            bytes: DEFAULT_BYTES,
            drop_slots: DEFAULT_DROP_SLOTS,
        }
    }

    /// Sets the backing buffer capacity in bytes.
    #[inline]
    #[must_use]
    pub const fn bytes(mut self, bytes: usize) -> Self {
        self.bytes = bytes;
        self
    }

    /// Sets the maximum number of `alloc_with_drop` registrations.
    ///
    /// Each registration consumes one slot of a fixed-size LIFO drop
    /// registry. Set to zero (the default) to forbid `alloc_with_drop`.
    #[inline]
    #[must_use]
    pub const fn drop_slots(mut self, drop_slots: usize) -> Self {
        self.drop_slots = drop_slots;
        self
    }

    /// Constructs the [`BumpAllocator`].
    ///
    /// # Errors
    ///
    /// Returns [`ArenaError::ZeroCapacity`] when `bytes` is zero.
    ///
    /// [`BumpAllocator`]: super::BumpAllocator
    pub fn build(self) -> Result<BumpAllocator, ArenaError> {
        BumpAllocator::from_builder(self.bytes, self.drop_slots)
    }
}

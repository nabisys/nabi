//! [`Slab`] generational slab allocator and [`SlabError`].

use core::fmt;
use core::mem::MaybeUninit;

use super::key::{Generation, SlabKey};

const FREE_SENTINEL: u32 = u32::MAX;

/// Errors emitted by [`Slab`] operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabError {
    /// All slots are occupied; `insert` cannot proceed.
    Full,
}

impl fmt::Display for SlabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => f.write_str("slab is full"),
        }
    }
}

impl core::error::Error for SlabError {}

struct Slot<T> {
    generation: Generation,
    next_free: u32,
    data: MaybeUninit<T>,
}

/// Per-worker generational slab with fixed capacity.
///
/// O(1) insert, get, and remove. Generational indices detect stale
/// handles when a slot is recycled.
///
/// Capacity is fixed at construction; `insert` returns
/// [`SlabError::Full`] when all slots are occupied. The backing storage
/// never reallocates.
///
/// The free list is in-band: empty slots store the next free index in
/// the slot itself, terminated by `u32::MAX`.
///
/// # Concurrency
///
/// `insert`, `remove`, and `get_mut` take `&mut self`, so only the
/// owning worker can mutate. This makes cross-worker insertion a
/// compile-time error rather than a runtime check.
///
/// # Examples
///
/// ```
/// use nabi_runtime::memory::slab::Slab;
///
/// let mut slab: Slab<u32> = Slab::new(4);
/// let Ok(key) = slab.insert(42) else { panic!("insert into fresh slab must succeed") };
/// assert_eq!(slab.get(key), Some(&42));
/// assert_eq!(slab.remove(key), Some(42));
/// assert_eq!(slab.get(key), None);
/// ```
pub struct Slab<T> {
    slots: Vec<Slot<T>>,
    free_head: u32,
    len: usize,
}

impl<T> Slab<T> {
    /// Creates a new slab with fixed `capacity`.
    ///
    /// All slots start empty and linked into the free list in index order.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` exceeds `u32::MAX as usize`.
    pub fn new(capacity: usize) -> Self {
        let Ok(capacity_u32) = u32::try_from(capacity) else {
            panic!("capacity {capacity} exceeds u32::MAX");
        };
        let mut slots = Vec::with_capacity(capacity);
        for idx in 0..capacity_u32 {
            let next_free = if idx + 1 < capacity_u32 {
                idx + 1
            } else {
                FREE_SENTINEL
            };
            slots.push(Slot {
                generation: Generation::ZERO,
                next_free,
                data: MaybeUninit::uninit(),
            });
        }
        let free_head = if capacity_u32 == 0 { FREE_SENTINEL } else { 0 };
        Self {
            slots,
            free_head,
            len: 0,
        }
    }

    /// Inserts `value`, returning a [`SlabKey`] that locates it.
    ///
    /// # Errors
    ///
    /// Returns [`SlabError::Full`] when all slots are currently occupied.
    pub fn insert(&mut self, value: T) -> Result<SlabKey, SlabError> {
        let idx = self.free_head;
        if idx == FREE_SENTINEL {
            return Err(SlabError::Full);
        }
        let slot = &mut self.slots[idx as usize];
        if slot.generation.is_occupied() {
            unreachable!("free list points to occupied slot {idx}");
        }
        let next = slot.next_free;
        slot.next_free = FREE_SENTINEL;
        slot.generation = slot.generation.next();
        slot.data.write(value);
        let generation = slot.generation;
        self.free_head = next;
        self.len += 1;
        Ok(SlabKey::new(idx, generation))
    }

    /// Returns a shared reference to the value at `key`, if present.
    pub fn get(&self, key: SlabKey) -> Option<&T> {
        let slot = self.slots.get(key.index() as usize)?;
        if slot.generation != key.generation() {
            return None;
        }
        // SAFETY: matching generation implies the slot is occupied; `data`
        // was initialised by `MaybeUninit::write` in `insert` and has not
        // been read out by `remove`. The shared borrow is bounded by
        // `&self`.
        Some(unsafe { slot.data.assume_init_ref() })
    }

    /// Returns an exclusive reference to the value at `key`, if present.
    pub fn get_mut(&mut self, key: SlabKey) -> Option<&mut T> {
        let slot = self.slots.get_mut(key.index() as usize)?;
        if slot.generation != key.generation() {
            return None;
        }
        // SAFETY: matching generation implies `data` is initialised. The
        // exclusive borrow is bounded by `&mut self`, so no aliasing
        // reference can coexist.
        Some(unsafe { slot.data.assume_init_mut() })
    }

    /// Removes and returns the value at `key`, if present.
    pub fn remove(&mut self, key: SlabKey) -> Option<T> {
        let head = self.free_head;
        let idx = key.index();
        let slot = self.slots.get_mut(idx as usize)?;
        if slot.generation != key.generation() {
            return None;
        }
        // SAFETY: matching generation implies `data` is initialised.
        // `assume_init_read` transfers ownership out; advancing the
        // generation immediately after marks the slot empty so future
        // reads through `data` are forbidden.
        let value = unsafe { slot.data.assume_init_read() };
        slot.generation = slot.generation.next();
        slot.next_free = head;
        self.free_head = idx;
        self.len -= 1;
        Some(value)
    }

    /// Returns an iterator over occupied slots as `(SlabKey, &T)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (SlabKey, &T)> + '_ {
        self.slots.iter().enumerate().filter_map(|(idx, slot)| {
            if !slot.generation.is_occupied() {
                return None;
            }
            // SAFETY: occupied parity implies the slot was written by
            // `insert` and not yet drained by `remove`.
            let value_ref = unsafe { slot.data.assume_init_ref() };
            #[allow(
                clippy::cast_possible_truncation,
                reason = "slots.len() is bounded by u32::MAX in `Slab::new`"
            )]
            let key = SlabKey::new(idx as u32, slot.generation);
            Some((key, value_ref))
        })
    }

    /// Returns the number of occupied slots.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns the fixed capacity.
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Returns `true` when no slots are occupied.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T> Drop for Slab<T> {
    fn drop(&mut self) {
        for slot in &mut self.slots {
            if slot.generation.is_occupied() {
                // SAFETY: occupied parity implies `data` is initialised.
                // Dropping in place releases owned resources before the
                // backing `Vec` frees the slot storage.
                unsafe { slot.data.assume_init_drop() };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_or_panic<T>(slab: &mut Slab<T>, value: T) -> SlabKey {
        match slab.insert(value) {
            Ok(key) => key,
            Err(SlabError::Full) => panic!("insert must succeed: slab unexpectedly full"),
        }
    }

    #[test]
    fn insert_get_roundtrip() {
        let mut slab: Slab<u32> = Slab::new(2);
        let key = insert_or_panic(&mut slab, 7);
        assert_eq!(slab.get(key), Some(&7));
        assert_eq!(slab.len(), 1);
    }

    #[test]
    fn remove_returns_value_and_reuses_slot() {
        let mut slab: Slab<u32> = Slab::new(2);
        let key = insert_or_panic(&mut slab, 11);
        assert_eq!(slab.remove(key), Some(11));
        assert!(slab.is_empty());
        let key2 = insert_or_panic(&mut slab, 22);
        assert_eq!(key2.index(), key.index());
        assert_ne!(key2.generation(), key.generation());
    }

    #[test]
    fn aba_stale_key_returns_none() {
        let mut slab: Slab<u32> = Slab::new(1);
        let key_a = insert_or_panic(&mut slab, 100);
        slab.remove(key_a);
        let key_b = insert_or_panic(&mut slab, 200);
        assert_eq!(key_a.index(), key_b.index());
        assert_eq!(slab.get(key_a), None);
        assert_eq!(slab.get(key_b), Some(&200));
    }

    #[test]
    fn full_capacity_returns_full_error() {
        let mut slab: Slab<u32> = Slab::new(2);
        insert_or_panic(&mut slab, 1);
        insert_or_panic(&mut slab, 2);
        assert_eq!(slab.insert(3), Err(SlabError::Full));
    }

    #[test]
    fn zero_capacity_immediately_full() {
        let mut slab: Slab<u32> = Slab::new(0);
        assert_eq!(slab.insert(1), Err(SlabError::Full));
        assert_eq!(slab.capacity(), 0);
    }

    #[test]
    fn get_mut_modifies_in_place() {
        let mut slab: Slab<u32> = Slab::new(1);
        let key = insert_or_panic(&mut slab, 0);
        let Some(slot) = slab.get_mut(key) else {
            panic!("just-inserted key must resolve");
        };
        *slot = 99;
        assert_eq!(slab.get(key), Some(&99));
    }

    #[test]
    fn iter_yields_only_occupied_in_index_order() {
        let mut slab: Slab<u32> = Slab::new(4);
        let k0 = insert_or_panic(&mut slab, 10);
        insert_or_panic(&mut slab, 20);
        insert_or_panic(&mut slab, 30);
        slab.remove(k0);
        let collected: Vec<u32> = slab.iter().map(|(_, v)| *v).collect();
        assert_eq!(collected.as_slice(), &[20u32, 30]);
    }

    #[test]
    fn iter_empty_after_full_drain() {
        let mut slab: Slab<u32> = Slab::new(2);
        let k = insert_or_panic(&mut slab, 1);
        slab.remove(k);
        assert!(slab.iter().next().is_none());
    }

    #[test]
    fn drop_runs_on_remaining_occupied_slots() {
        use core::cell::Cell;
        struct Bomb<'a>(&'a Cell<usize>);
        impl Drop for Bomb<'_> {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let counter = Cell::new(0usize);
        {
            let mut slab: Slab<Bomb<'_>> = Slab::new(3);
            insert_or_panic(&mut slab, Bomb(&counter));
            let k = insert_or_panic(&mut slab, Bomb(&counter));
            insert_or_panic(&mut slab, Bomb(&counter));
            slab.remove(k);
        }
        assert_eq!(counter.get(), 3);
    }

    #[test]
    fn len_tracks_inserts_and_removals() {
        let mut slab: Slab<u32> = Slab::new(3);
        assert_eq!(slab.len(), 0);
        let k1 = insert_or_panic(&mut slab, 1);
        insert_or_panic(&mut slab, 2);
        assert_eq!(slab.len(), 2);
        slab.remove(k1);
        assert_eq!(slab.len(), 1);
    }

    #[test]
    fn out_of_range_index_returns_none() {
        let mut slab: Slab<u32> = Slab::new(2);
        insert_or_panic(&mut slab, 1);
        let stale = SlabKey::new(99, Generation(1));
        assert_eq!(slab.get(stale), None);
    }

    #[test]
    fn slaberror_display_message() {
        assert_eq!(SlabError::Full.to_string(), "slab is full");
    }
}

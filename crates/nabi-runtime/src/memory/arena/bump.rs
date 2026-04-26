//! [`BumpAllocator`] — fixed-capacity bump allocator with LIFO drop registry.

use core::alloc::Layout;
use core::fmt;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use nabi_core::FlatLayout;

use super::super::generation::Generation;
use super::builder::BumpAllocatorBuilder;
use super::phase::ArenaPhase;

/// Backing-buffer alignment. Matches typical libc malloc on 64-bit
/// targets and bounds the largest `T::align` an arena can satisfy.
const MAX_ALIGN: usize = 16;

/// Errors emitted by [`BumpAllocator`] operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArenaError {
    /// Builder requested zero bytes.
    ZeroCapacity,
    /// Allocation request exceeds remaining capacity.
    Exhausted {
        /// Bytes the failed call asked for (including alignment padding).
        requested: usize,
        /// Bytes still available before the failure.
        available: usize,
    },
    /// `alloc_with_drop` called but the drop registry is full.
    DropRegistryFull {
        /// Configured drop registry capacity.
        capacity: usize,
    },
    /// Allocation attempted in the Frozen phase.
    WrongPhase {
        /// Phase observed at the call site.
        current: ArenaPhase,
    },
}

impl fmt::Display for ArenaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCapacity => f.write_str("arena builder requested zero bytes"),
            Self::Exhausted {
                requested,
                available,
            } => write!(
                f,
                "arena exhausted: requested {requested}, available {available}"
            ),
            Self::DropRegistryFull { capacity } => {
                write!(f, "arena drop registry full (capacity {capacity})")
            }
            Self::WrongPhase { current } => write!(
                f,
                "arena allocation requires Build phase (current {current:?})"
            ),
        }
    }
}

impl core::error::Error for ArenaError {}

struct DropEntry {
    ptr: *mut u8,
    drop_fn: unsafe fn(*mut u8),
}

/// Fixed-capacity bump allocator for Conductor DAG-scoped data.
///
/// Two allocation paths:
///
/// - [`alloc`] — fast path for `T: FlatLayout`; nothing to drop, so
///   `reset()` reclaims the whole region in O(1).
/// - [`alloc_with_drop`] — registers a destructor in a fixed-size LIFO,
///   invoked in reverse order on `reset()` and on `Drop`.
///
/// Lifecycle: `Build` → `freeze()` → `Frozen` → `reset(outstanding)` →
/// `Build`. Allocation is permitted only in Build.
///
/// `reset(outstanding: u32)` panics if `outstanding != 0`. Outstanding
/// task accounting is the Conductor's responsibility — the arena does
/// not track it itself, and this is intentional (no atomics on the
/// reset path).
///
/// # Panics during drop
///
/// If a registered drop function panics during `reset()` or `Drop`, the
/// process aborts. This matches the std `Vec` / `HashMap` policy.
///
/// [`alloc`]: BumpAllocator::alloc
/// [`alloc_with_drop`]: BumpAllocator::alloc_with_drop
pub struct BumpAllocator {
    buffer: NonNull<u8>,
    capacity: usize,
    cursor: usize,
    drops: Vec<DropEntry>,
    drop_capacity: usize,
    phase: ArenaPhase,
    generation: Generation,
}

impl BumpAllocator {
    /// Returns a fresh [`BumpAllocatorBuilder`].
    #[inline]
    #[must_use]
    pub const fn builder() -> BumpAllocatorBuilder {
        BumpAllocatorBuilder::new()
    }

    pub(super) fn from_builder(bytes: usize, drop_slots: usize) -> Result<Self, ArenaError> {
        if bytes == 0 {
            return Err(ArenaError::ZeroCapacity);
        }
        let layout = buffer_layout(bytes)?;
        // SAFETY: layout has nonzero size (bytes > 0 checked above) and
        // a valid power-of-two alignment (MAX_ALIGN). The returned
        // region is owned exclusively by this allocator until Drop.
        let raw = unsafe { std::alloc::alloc(layout) };
        let Some(buffer) = NonNull::new(raw) else {
            std::alloc::handle_alloc_error(layout);
        };
        Ok(Self {
            buffer,
            capacity: bytes,
            cursor: 0,
            drops: Vec::with_capacity(drop_slots),
            drop_capacity: drop_slots,
            phase: ArenaPhase::Build,
            generation: Generation::ZERO,
        })
    }

    fn alloc_raw(&mut self, layout: Layout) -> Result<NonNull<u8>, ArenaError> {
        if self.phase != ArenaPhase::Build {
            return Err(ArenaError::WrongPhase {
                current: self.phase,
            });
        }
        if layout.align() > MAX_ALIGN {
            return Err(self.exhausted(layout.size() + layout.align()));
        }
        let aligned = align_up(self.cursor, layout.align());
        let new_cursor = aligned
            .checked_add(layout.size())
            .ok_or_else(|| self.exhausted(layout.size()))?;
        if new_cursor > self.capacity {
            return Err(self.exhausted(new_cursor - self.cursor));
        }
        // SAFETY: `aligned < capacity` (checked above) and `buffer` has
        // provenance over the entire region returned by std::alloc;
        // `add` preserves that provenance.
        let ptr = unsafe { self.buffer.as_ptr().add(aligned) };
        self.cursor = new_cursor;
        // SAFETY: `buffer` is non-null and `add` of an in-bounds offset
        // cannot produce null.
        Ok(unsafe { NonNull::new_unchecked(ptr) })
    }

    #[inline]
    const fn exhausted(&self, requested: usize) -> ArenaError {
        ArenaError::Exhausted {
            requested,
            available: self.capacity - self.cursor,
        }
    }

    /// Allocates a `T: FlatLayout` value, returning a typed pointer.
    ///
    /// # Errors
    ///
    /// - [`ArenaError::WrongPhase`] when called outside Build.
    /// - [`ArenaError::Exhausted`] when remaining capacity cannot fit
    ///   the value (including alignment padding).
    pub fn alloc<T: FlatLayout>(&mut self, value: T) -> Result<NonNull<T>, ArenaError> {
        let layout = Layout::new::<T>();
        let raw = self.alloc_raw(layout)?;
        let typed = raw.cast::<T>();
        // SAFETY: `alloc_raw` returned a writable, aligned pointer with
        // backing storage valid for `size_of::<T>()` bytes.
        unsafe { typed.as_ptr().write(value) };
        Ok(typed)
    }

    /// Allocates a value of any `T: Send` and registers its destructor.
    ///
    /// The drop function is invoked in LIFO order on the next `reset()`
    /// and on `Drop`. The `Send` bound is what makes `BumpAllocator`'s
    /// own `Send` impl sound — registering a `!Send` value would let it
    /// follow the allocator across a thread boundary and run its drop
    /// on the wrong thread.
    ///
    /// # Errors
    ///
    /// - [`ArenaError::WrongPhase`] when called outside Build.
    /// - [`ArenaError::Exhausted`] when the buffer cannot fit `T`.
    /// - [`ArenaError::DropRegistryFull`] when the drop registry is at
    ///   capacity.
    pub fn alloc_with_drop<T: Send>(&mut self, value: T) -> Result<NonNull<T>, ArenaError> {
        if self.drops.len() >= self.drop_capacity {
            return Err(ArenaError::DropRegistryFull {
                capacity: self.drop_capacity,
            });
        }
        let layout = Layout::new::<T>();
        let raw = self.alloc_raw(layout)?;
        let typed = raw.cast::<T>();
        // SAFETY: same as `alloc` — aligned, owned, big enough for T.
        unsafe { typed.as_ptr().write(value) };
        self.drops.push(DropEntry {
            ptr: typed.as_ptr().cast::<u8>(),
            drop_fn: drop_in_place_for::<T>,
        });
        Ok(typed)
    }

    /// Allocates an uninitialised slice of `count` `T: FlatLayout`.
    ///
    /// # Errors
    ///
    /// - [`ArenaError::WrongPhase`] when called outside Build.
    /// - [`ArenaError::Exhausted`] when the buffer cannot fit the slice
    ///   or `count * size_of::<T>()` overflows.
    pub fn alloc_slice<T: FlatLayout>(
        &mut self,
        count: usize,
    ) -> Result<NonNull<[MaybeUninit<T>]>, ArenaError> {
        let bytes = core::mem::size_of::<T>()
            .checked_mul(count)
            .ok_or_else(|| self.exhausted(usize::MAX))?;
        let layout = Layout::from_size_align(bytes, core::mem::align_of::<T>())
            .map_err(|_| self.exhausted(bytes))?;
        let raw = self.alloc_raw(layout)?;
        let typed = raw.cast::<MaybeUninit<T>>();
        Ok(NonNull::slice_from_raw_parts(typed, count))
    }

    /// Transitions Build → Frozen.
    ///
    /// Subsequent `alloc*` calls return [`ArenaError::WrongPhase`].
    pub const fn freeze(&mut self) {
        self.phase = ArenaPhase::Frozen;
    }

    /// Transitions Frozen → Build, dropping every registered destructor
    /// in LIFO order and bumping the generation.
    ///
    /// # Panics
    ///
    /// Panics if `outstanding != 0`. The Conductor guarantees the
    /// in-flight count is zero before resetting.
    pub fn reset(&mut self, outstanding: u32) {
        assert!(
            outstanding == 0,
            "arena reset called with {outstanding} outstanding handles"
        );
        self.run_drops();
        self.cursor = 0;
        self.phase = ArenaPhase::Build;
        self.generation = self.generation.next();
    }

    /// Returns the current lifecycle phase.
    #[inline]
    #[must_use]
    pub const fn phase(&self) -> ArenaPhase {
        self.phase
    }

    /// Returns the current generation.
    #[inline]
    #[must_use]
    pub const fn generation(&self) -> Generation {
        self.generation
    }

    /// Returns the number of bytes consumed in the active phase.
    #[inline]
    #[must_use]
    pub const fn used(&self) -> usize {
        self.cursor
    }

    /// Returns the total backing capacity in bytes.
    #[inline]
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns bytes still available in the active phase.
    #[inline]
    #[must_use]
    pub const fn available(&self) -> usize {
        self.capacity - self.cursor
    }

    fn run_drops(&mut self) {
        while let Some(entry) = self.drops.pop() {
            // SAFETY: each entry was registered by `alloc_with_drop` for
            // a value of the type whose monomorphised
            // `drop_in_place_for` is stored in `entry.drop_fn`. The
            // pointer is valid until consumed here, and `drop_in_place`
            // is the canonical destructor invocation.
            unsafe { (entry.drop_fn)(entry.ptr) };
        }
    }
}

unsafe fn drop_in_place_for<T>(ptr: *mut u8) {
    // SAFETY: caller guarantees `ptr` was produced by
    // `alloc_with_drop::<T>` and has not been read out since
    // registration.
    unsafe { ptr.cast::<T>().drop_in_place() };
}

impl Drop for BumpAllocator {
    fn drop(&mut self) {
        self.run_drops();
        let Ok(layout) = Layout::from_size_align(self.capacity, MAX_ALIGN) else {
            return;
        };
        // SAFETY: `buffer` was allocated by `std::alloc::alloc` with this
        // exact layout in `from_builder` and has not been freed since;
        // the allocator owns it exclusively.
        unsafe { std::alloc::dealloc(self.buffer.as_ptr(), layout) };
    }
}

#[allow(
    clippy::non_send_fields_in_send_ty,
    reason = "DropEntry pointers alias storage owned by this allocator; the Send obligation on registered values is enforced at compile time by the T: Send bound on alloc_with_drop"
)]
// SAFETY: `BumpAllocator` owns its `NonNull<u8>` backing buffer and
// drop registry outright — no shared state escapes. The `*mut u8`
// fields inside `DropEntry` only point into the same owned region,
// so transferring the allocator transfers the pointed-to data with
// it. The `T: Send` bound on `alloc_with_drop` ensures every
// registered drop_fn is sound to invoke on whatever thread now owns
// the allocator.
unsafe impl Send for BumpAllocator {}

#[inline]
const fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

#[inline]
fn buffer_layout(bytes: usize) -> Result<Layout, ArenaError> {
    Layout::from_size_align(bytes, MAX_ALIGN).map_err(|_| ArenaError::Exhausted {
        requested: bytes,
        available: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_or_panic(bytes: usize, drops: usize) -> BumpAllocator {
        let Ok(arena) = BumpAllocator::builder()
            .bytes(bytes)
            .drop_slots(drops)
            .build()
        else {
            panic!("builder must succeed for valid bytes/drops")
        };
        arena
    }

    #[test]
    fn builder_default_rejects_alloc_with_drop() {
        let mut arena = build_or_panic(1024, 0);
        assert_eq!(
            arena.alloc_with_drop(42u32).err(),
            Some(ArenaError::DropRegistryFull { capacity: 0 })
        );
    }

    #[test]
    fn builder_zero_bytes_returns_zero_capacity() {
        assert_eq!(
            BumpAllocator::builder().bytes(0).build().err(),
            Some(ArenaError::ZeroCapacity)
        );
    }

    #[test]
    fn alloc_flatlayout_roundtrip() {
        let mut arena = build_or_panic(64, 0);
        let Ok(ptr) = arena.alloc::<u32>(0xDEAD_BEEF) else {
            panic!("alloc must succeed in 64-byte arena")
        };
        // SAFETY: pointer was just written and lives until arena drops.
        let value = unsafe { ptr.as_ptr().read() };
        assert_eq!(value, 0xDEAD_BEEF);
    }

    #[test]
    fn alloc_with_drop_invokes_drop_in_lifo_on_reset() {
        use core::cell::RefCell;
        std::thread_local! {
            static LIFO_LOG: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
        }
        struct Recorder {
            tag: u32,
        }
        impl Drop for Recorder {
            fn drop(&mut self) {
                LIFO_LOG.with(|log| log.borrow_mut().push(self.tag));
            }
        }
        LIFO_LOG.with(|log| log.borrow_mut().clear());
        let mut arena = build_or_panic(256, 4);
        let Ok(_) = arena.alloc_with_drop(Recorder { tag: 1 }) else {
            panic!("first alloc_with_drop must succeed")
        };
        let Ok(_) = arena.alloc_with_drop(Recorder { tag: 2 }) else {
            panic!("second alloc_with_drop must succeed")
        };
        let Ok(_) = arena.alloc_with_drop(Recorder { tag: 3 }) else {
            panic!("third alloc_with_drop must succeed")
        };
        arena.freeze();
        arena.reset(0);
        LIFO_LOG.with(|log| assert_eq!(log.borrow().as_slice(), &[3u32, 2, 1]));
    }

    #[test]
    fn alloc_in_frozen_returns_wrong_phase() {
        let mut arena = build_or_panic(64, 0);
        arena.freeze();
        assert_eq!(
            arena.alloc::<u32>(7).err(),
            Some(ArenaError::WrongPhase {
                current: ArenaPhase::Frozen
            })
        );
    }

    #[test]
    fn reset_zero_returns_to_build() {
        let mut arena = build_or_panic(64, 0);
        arena.freeze();
        arena.reset(0);
        assert_eq!(arena.phase(), ArenaPhase::Build);
    }

    #[test]
    #[should_panic(expected = "outstanding")]
    fn reset_nonzero_panics() {
        let mut arena = build_or_panic(64, 0);
        arena.freeze();
        arena.reset(1);
    }

    #[test]
    fn reset_bumps_generation() {
        let mut arena = build_or_panic(64, 0);
        let g0 = arena.generation();
        arena.freeze();
        arena.reset(0);
        assert_ne!(g0, arena.generation());
    }

    #[test]
    fn exhausted_returns_error_with_sizes() {
        let mut arena = build_or_panic(8, 0);
        let Ok(_) = arena.alloc::<u64>(1) else {
            panic!("first u64 must fit in 8-byte arena")
        };
        let Err(err) = arena.alloc::<u32>(2) else {
            panic!("second alloc must exhaust the arena")
        };
        assert!(matches!(err, ArenaError::Exhausted { .. }));
    }

    #[test]
    fn drop_slot_capacity_enforced() {
        let mut arena = build_or_panic(256, 1);
        let Ok(_) = arena.alloc_with_drop(0u32) else {
            panic!("first alloc_with_drop must succeed")
        };
        assert_eq!(
            arena.alloc_with_drop(1u32).err(),
            Some(ArenaError::DropRegistryFull { capacity: 1 })
        );
    }

    #[test]
    fn drop_invokes_pending_drops_on_drop() {
        use core::cell::Cell;
        std::thread_local! {
            static DROP_COUNT: Cell<usize> = const { Cell::new(0) };
        }
        struct Bomb;
        impl Drop for Bomb {
            fn drop(&mut self) {
                DROP_COUNT.with(|c| c.set(c.get() + 1));
            }
        }
        DROP_COUNT.with(|c| c.set(0));
        {
            let mut arena = build_or_panic(64, 2);
            let Ok(_) = arena.alloc_with_drop(Bomb) else {
                panic!("first alloc_with_drop must succeed")
            };
            let Ok(_) = arena.alloc_with_drop(Bomb) else {
                panic!("second alloc_with_drop must succeed")
            };
        }
        assert_eq!(DROP_COUNT.with(Cell::get), 2);
    }

    #[test]
    fn used_and_available_track_cursor() {
        let mut arena = build_or_panic(16, 0);
        assert_eq!(arena.used(), 0);
        assert_eq!(arena.available(), 16);
        let Ok(_) = arena.alloc::<u32>(0) else {
            panic!("u32 must fit in 16-byte arena")
        };
        assert_eq!(arena.used(), 4);
        assert_eq!(arena.available(), 12);
    }

    #[test]
    fn arenaerror_display_messages() {
        assert_eq!(
            ArenaError::ZeroCapacity.to_string(),
            "arena builder requested zero bytes"
        );
        assert_eq!(
            ArenaError::DropRegistryFull { capacity: 4 }.to_string(),
            "arena drop registry full (capacity 4)"
        );
    }
}

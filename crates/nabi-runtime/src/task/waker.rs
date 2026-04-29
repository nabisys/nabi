//! [`waker_from_task_ref`] — turns a [`TaskRef`] into a [`Waker`] without
//! allocation by smuggling the 64-bit handle through the `RawWaker` data
//! pointer slot.
//!
//! Strict-provenance: the data pointer is built with
//! [`core::ptr::without_provenance`] so the integer round-trip is sound on
//! targets that distinguish address-only and provenance-carrying pointers.
//!
//! Wake handoff is gated on the worker scheduler. Until that lands the
//! [`wake_fn`] entry point traps via `unimplemented!` — the runtime hands out
//! wakers to drive `Future::poll`, but no production code calls `wake()` on
//! them yet.
#![allow(
    clippy::redundant_pub_crate,
    reason = "satisfies the workspace `unreachable_pub` lint on a private module"
)]
#![allow(dead_code, reason = "no non-test caller in this revision")]

use core::ptr;
use core::task::{RawWaker, RawWakerVTable, Waker};

use crate::task::TaskRef;

/// `nabi` packs a 64-bit `TaskRef` into the `RawWaker` data pointer. That
/// only works on 64-bit targets where `usize == u64`. Earlier failure here
/// is preferable to a silent truncation at runtime.
const _: () = assert!(
    usize::BITS == 64,
    "nabi-runtime requires a 64-bit target so TaskRef fits in *const ()",
);

/// Builds a [`Waker`] whose `RawWaker::data` encodes the given [`TaskRef`].
///
/// The returned waker is `Clone + Send + Sync` per stdlib contract; clones
/// are zero-cost integer copies.
pub(crate) fn waker_from_task_ref(task_ref: TaskRef) -> Waker {
    let data = task_ref_to_data(task_ref);
    // SAFETY: `VTABLE` is a process-wide `'static` table and `data` is built
    // by `task_ref_to_data` to encode a valid `TaskRef`; the four vtable
    // entries treat it strictly as an integer round-trip.
    unsafe { Waker::from_raw(RawWaker::new(data, &VTABLE)) }
}

/// Converts a [`TaskRef`] into the integer-shaped data pointer that the
/// vtable will round-trip back.
#[inline]
#[allow(
    clippy::cast_possible_truncation,
    reason = "module-level static assert pins usize::BITS == 64, so u64 -> usize is lossless"
)]
const fn task_ref_to_data(task_ref: TaskRef) -> *const () {
    let bits = task_ref.raw() as usize;
    ptr::without_provenance(bits)
}

/// Recovers a [`TaskRef`] from a vtable callback's `data` argument.
///
/// Not `const` because `<*const ()>::addr` is currently a non-const method.
#[inline]
fn data_to_task_ref(data: *const ()) -> TaskRef {
    let bits = data.addr() as u64;
    TaskRef::from_raw(bits)
}

/// Process-wide vtable shared by every `nabi` task waker.
static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_fn, wake_fn, wake_by_ref_fn, drop_fn);

/// Clone callback: integer copy of the data pointer plus the same vtable.
unsafe fn clone_fn(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}

/// Consume-by-value wake. Traps until the worker scheduler injector lands —
/// the runtime hands out wakers but no in-tree caller invokes `wake()` yet.
#[allow(
    clippy::unimplemented,
    reason = "wake handoff depends on the worker scheduler injector, which is not yet wired"
)]
unsafe fn wake_fn(data: *const ()) {
    let task_ref = data_to_task_ref(data);
    unimplemented!("wake_fn requires the worker scheduler injector; called for {task_ref:?}");
}

/// Wake by reference. Same staging as [`wake_fn`].
#[allow(
    clippy::unimplemented,
    reason = "wake handoff depends on the worker scheduler injector, which is not yet wired"
)]
unsafe fn wake_by_ref_fn(data: *const ()) {
    let task_ref = data_to_task_ref(data);
    unimplemented!(
        "wake_by_ref_fn requires the worker scheduler injector; called for {task_ref:?}"
    );
}

/// Drop callback: no-op because [`TaskRef`] is `Copy` and the data pointer
/// is just an integer.
const unsafe fn drop_fn(_data: *const ()) {}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::memory::Generation;

    fn fake_ref(worker_id: u8, generation: u32, index: u32) -> TaskRef {
        TaskRef::from_arena(worker_id, index, Generation(generation))
    }

    #[test]
    fn waker_from_task_ref_round_trips_through_data_pointer() {
        let original = fake_ref(7, 42, 0xDEAD_BEEF);
        let waker = waker_from_task_ref(original);
        // The Waker exposes its data via Waker::data() (stable since 1.83).
        let recovered = data_to_task_ref(waker.data());
        assert_eq!(recovered, original);
    }

    #[test]
    fn cloned_waker_preserves_task_ref() {
        let original = fake_ref(3, 1, 99);
        let waker = waker_from_task_ref(original);
        let cloned = waker.clone();
        assert!(waker.will_wake(&cloned));
        let recovered = data_to_task_ref(cloned.data());
        assert_eq!(recovered, original);
    }

    #[test]
    fn waker_will_wake_matches_for_independent_constructions() {
        let task_ref = fake_ref(0, 1, 1);
        let a = waker_from_task_ref(task_ref);
        let b = waker_from_task_ref(task_ref);
        assert!(a.will_wake(&b));
    }

    #[test]
    fn waker_will_wake_distinguishes_distinct_refs() {
        let a = waker_from_task_ref(fake_ref(0, 1, 1));
        let b = waker_from_task_ref(fake_ref(0, 1, 2));
        assert!(!a.will_wake(&b));
    }

    #[test]
    fn drop_fn_is_a_noop() {
        // Drop the waker — covered by the test simply running without
        // leaking under miri / sanitizer.
        let _ = waker_from_task_ref(fake_ref(0, 1, 1));
    }

    #[test]
    #[should_panic(expected = "wake_fn requires the worker scheduler")]
    fn wake_traps_until_scheduler_lands() {
        let waker = waker_from_task_ref(fake_ref(0, 1, 1));
        waker.wake();
    }

    #[test]
    #[should_panic(expected = "wake_by_ref_fn requires the worker scheduler")]
    fn wake_by_ref_traps_until_scheduler_lands() {
        let waker = waker_from_task_ref(fake_ref(0, 1, 1));
        waker.wake_by_ref();
    }
}

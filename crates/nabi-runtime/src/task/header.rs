//! [`TaskHeader`], [`TaskVTable`], and the [`Slot<F>`] / [`Cell<F>`] memory
//! layout that backs every spawned task.
//!
//! `repr(C)` pins field order so a worker can cast a `*mut TaskHeader` back to
//! `*mut Slot<F>` (offset 0) when invoking the type-erased vtable, and so the
//! join handle can read the output at a fixed `OUTPUT_OFFSET = sizeof::<TaskHeader>()`.
#![allow(
    clippy::redundant_pub_crate,
    reason = "satisfies the workspace `unreachable_pub` lint on a private module"
)]
#![allow(dead_code, reason = "no non-test caller in this revision")]

use core::alloc::Layout;
use core::future::Future;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::ptr::{self, NonNull};
use core::task::{Context, Poll};

use nabi_core::id::Nid;
use nabi_core::namespace::Namespace;

use crate::task::TaskRef;
use crate::task::state::{AtomicTaskState, TaskState};

/// Per-task control block. The first field of every [`Slot<F>`].
///
/// `repr(C)` is load-bearing: the runtime relies on `*mut Slot<F>` and
/// `*mut TaskHeader` aliasing at offset 0, and on the trailing
/// `Cell<F>` starting exactly `size_of::<TaskHeader>()` bytes past the
/// header (see [`Slot::OUTPUT_OFFSET`]).
///
/// # Field semantics
///
/// * `state` — atomic CAS-based lifecycle (see [`AtomicTaskState`]).
/// * `nid` — observability identity, propagated to children via `Nid::child`.
/// * `namespace` — logical scope; the per-process interning is decided at the
///   call site, not here.
/// * `first_child` / `next_sibling` — intrusive children list, manipulated by
///   helpers in [`crate::task::children`]. Both are [`Option<TaskRef>`] so
///   absence is type-level rather than encoded as a sentinel index.
/// * `vtable` — the type-erased entry points for `poll` and `drop_in_place`,
///   stamped per `F` by [`Slot::VTABLE`].
///
/// # Examples
///
/// ```text
/// // Internal use within nabi-runtime.
/// let header = TaskHeader::new(Nid::detached(), Namespace::ROOT, &Slot::<F>::VTABLE);
/// assert_eq!(header.state.load(), TaskState::Sleeping);
/// ```
#[derive(Debug)]
#[repr(C)]
pub(crate) struct TaskHeader {
    pub(crate) state: AtomicTaskState,
    pub(crate) nid: Nid,
    pub(crate) namespace: Namespace,
    pub(crate) first_child: Option<TaskRef>,
    pub(crate) next_sibling: Option<TaskRef>,
    pub(crate) vtable: &'static TaskVTable,
}

impl TaskHeader {
    /// Constructs a new header in `Sleeping` with no children.
    #[cfg(not(loom))]
    #[inline]
    pub(crate) const fn new(nid: Nid, namespace: Namespace, vtable: &'static TaskVTable) -> Self {
        Self {
            state: AtomicTaskState::new(),
            nid,
            namespace,
            first_child: None,
            next_sibling: None,
            vtable,
        }
    }

    /// Loom variant — `loom::sync::atomic::AtomicU8::new` is not available in
    /// const context, so the const qualifier is dropped under `--cfg loom`.
    #[cfg(loom)]
    #[inline]
    pub(crate) fn new(nid: Nid, namespace: Namespace, vtable: &'static TaskVTable) -> Self {
        Self {
            state: AtomicTaskState::new(),
            nid,
            namespace,
            first_child: None,
            next_sibling: None,
            vtable,
        }
    }
}

/// Type-erased entry points for a task. One static instance per `F`,
/// reachable via [`Slot::VTABLE`].
///
/// Both entry points are safe-by-signature `fn` pointers. The runtime contract
/// — "the [`NonNull<TaskHeader>`] argument must point to a live [`Slot<F>`]
/// whose stamped vtable equals [`Slot::VTABLE`], with provenance covering the
/// entire slot" — is documented on each function and lives at the call site
/// that constructs the [`NonNull`]. Per workspace policy, the `unsafe`
/// surface is scoped to the actual unsafe operations inside each body, not
/// hoisted to the function signature.
#[derive(Debug)]
#[repr(C)]
pub(crate) struct TaskVTable {
    /// Polls the task's future once. Caller must have transitioned the
    /// header state to [`TaskState::Running`] before invoking.
    pub(crate) poll: fn(NonNull<TaskHeader>, &mut Context<'_>) -> Poll<()>,
    /// Drops the still-live half of the cell — `future` if poll has not
    /// returned `Ready`, otherwise `output`. Decides which by reading
    /// `state` with `Acquire` ordering.
    pub(crate) drop_in_place: fn(NonNull<TaskHeader>),
    /// Allocation layout of the entire `Slot<F>` (header + cell). Used by
    /// the slab/arena allocator at spawn time and by `dealloc` paths.
    pub(crate) layout: Layout,
}

/// Cell holding the future and (eventually) its output.
///
/// `repr(C)` puts `output` first so the join handle can read it at the fixed
/// offset [`Slot::OUTPUT_OFFSET`] without consulting `offset_of!`. The cell
/// is *not* an enum: which half is currently initialised is encoded by the
/// header's [`TaskState`] (Pending / Done / Taken) per the layout contract.
#[repr(C)]
pub(crate) struct Cell<F: Future> {
    /// Output written exactly once by `poll_fn` when the future returns
    /// `Ready`. Read at most once by the join handle's `Done → Taken`
    /// transition. Uninitialised before `Done`.
    pub(crate) output: MaybeUninit<F::Output>,
    /// The future itself. Live while state ∈ {Sleeping, Woken, Running};
    /// `drop_in_place_fn` consumes it on `Ready` or any cancellation path.
    pub(crate) future: F,
}

/// Concrete allocation unit. `Slot<F>` is what the slab or arena owns; the
/// runtime hands out `*mut TaskHeader` (offset 0) for type-erased work.
#[repr(C)]
pub(crate) struct Slot<F: Future> {
    pub(crate) header: TaskHeader,
    pub(crate) cell: Cell<F>,
    /// Marker so `Slot<F>` is invariant in `F` even when fields are
    /// constructed via raw pointer manipulation in tests.
    _marker: PhantomData<F>,
}

impl<F: Future> Slot<F> {
    /// Byte offset of `Cell<F>::output` from the start of the header.
    ///
    /// Equals `size_of::<TaskHeader>()` because `Slot<F>` is `repr(C)` with
    /// `header` first and `Cell<F>` second, and `Cell<F>` is `repr(C)` with
    /// `output` first. The join handle reads at this offset using
    /// `Handle<T>` knowledge of `F::Output`.
    pub(crate) const OUTPUT_OFFSET: usize = size_of::<TaskHeader>();

    /// Compile-time guard that `size_of::<Slot<F>>() <= 512`. The runtime
    /// enforces this for every concrete `F` by referencing the const, which
    /// triggers monomorphisation-time evaluation.
    pub(crate) const SIZE_OK: () = assert!(
        size_of::<Self>() <= 512,
        "Slot<F> exceeds 512 bytes; reduce sizeof::<F>() + sizeof::<F::Output>()",
    );

    /// Per-`F` vtable. The compiler stamps one static per concrete `F`.
    pub(crate) const VTABLE: TaskVTable = TaskVTable {
        poll: Self::poll_fn,
        drop_in_place: Self::drop_in_place_fn,
        layout: Layout::new::<Self>(),
    };

    /// Constructs an in-place `Slot<F>` from raw parts. The `state` starts at
    /// `Sleeping`, `output` is uninitialised, `first_child`/`next_sibling`
    /// are `None`.
    ///
    /// Caller is responsible for placing the resulting value into a slab or
    /// arena slot — this helper does not allocate.
    #[cfg(not(loom))]
    pub(crate) const fn new(nid: Nid, namespace: Namespace, future: F) -> Self {
        // Touch SIZE_OK so the compile-time bound is checked for every `F`
        // that ever instantiates `Slot::new`.
        let () = Self::SIZE_OK;
        Self {
            header: TaskHeader::new(nid, namespace, &Self::VTABLE),
            cell: Cell {
                output: MaybeUninit::uninit(),
                future,
            },
            _marker: PhantomData,
        }
    }

    #[cfg(loom)]
    pub(crate) fn new(nid: Nid, namespace: Namespace, future: F) -> Self {
        let () = Self::SIZE_OK;
        Self {
            header: TaskHeader::new(nid, namespace, &Self::VTABLE),
            cell: Cell {
                output: MaybeUninit::uninit(),
                future,
            },
            _marker: PhantomData,
        }
    }

    /// vtable entry: poll the task's future once.
    ///
    /// The signature is a safe `fn` pointer per workspace anti-patterns
    /// rule (no `unsafe fn` on entire functions); the runtime contract
    /// below is the responsibility of whoever constructs the [`NonNull`]
    /// at the call site.
    ///
    /// # Contract (caller-side)
    ///
    /// * `ptr` must point to the `TaskHeader` of a live `Slot<F>` whose
    ///   stamped vtable equals [`Self::VTABLE`].
    /// * `ptr`'s provenance must cover the entire `Slot<F>` allocation,
    ///   not just the leading `TaskHeader` prefix. Construct via
    ///   `NonNull::from(&mut slot).cast::<TaskHeader>()` or equivalent —
    ///   *never* `NonNull::from(&mut slot.header)`, which retags only
    ///   `[0..size_of::<TaskHeader>()]` and triggers a Stacked Borrows
    ///   violation when this function later reaches `cell.future`.
    /// * The header's state must be [`TaskState::Running`] at entry — the
    ///   caller (worker) performs the `Sleeping/Woken → Running` CAS before
    ///   invoking this function.
    /// * The cell's `future` half must still be initialised; that is the
    ///   case unless a previous `poll_fn` call returned `Poll::Ready`.
    ///
    /// On `Poll::Ready`, the function writes the output into `cell.output`,
    /// drops the future in place, and returns `Poll::Ready(())`. State
    /// transition `Running → Done` is the caller's responsibility, leaving
    /// the order CAS-after-write so the output is visible to any thread that
    /// observes `Done` with `Acquire`.
    fn poll_fn(ptr: NonNull<TaskHeader>, cx: &mut Context<'_>) -> Poll<()> {
        let slot: *mut Self = ptr.as_ptr().cast();
        // SAFETY: `cell` is a live field of `*slot`; we never form a `&mut`
        // to overlapping data while this raw reference is in use, and
        // `ptr`'s provenance covers the full slot per the caller contract.
        let cell_ptr: *mut Cell<F> = unsafe { &raw mut (*slot).cell };
        // SAFETY: the future is structurally pinned — it lives in the slab
        // slot (or arena cell) at a stable address for the duration of the
        // task, and the caller holds exclusive access for the poll cycle.
        let future_ref: &mut F = unsafe { &mut (*cell_ptr).future };
        // SAFETY: `future_ref` points at a structurally-pinned location; we
        // never move out of it before the poll completes.
        let future_pin: Pin<&mut F> = unsafe { Pin::new_unchecked(future_ref) };
        match future_pin.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(out) => {
                // SAFETY: state is `Running`, so `output` is still uninit and
                // no other thread can observe a partially-written slot.
                unsafe {
                    (*cell_ptr).output.write(out);
                }
                // SAFETY: future is no longer accessed after this call.
                // `Ready` is reached at most once per task, so this is the
                // sole drop site for the future on the Ready path.
                unsafe {
                    ptr::drop_in_place(&raw mut (*cell_ptr).future);
                }
                Poll::Ready(())
            }
        }
    }

    /// vtable entry: drop the still-live half of the cell.
    ///
    /// Same safe-by-signature `fn` shape as [`Self::poll_fn`]; the runtime
    /// contract is enforced at the call site that builds the [`NonNull`].
    ///
    /// # Contract (caller-side)
    ///
    /// * `ptr` must point to the `TaskHeader` of a live `Slot<F>` whose
    ///   stamped vtable equals [`Self::VTABLE`].
    /// * `ptr`'s provenance must cover the entire `Slot<F>` allocation,
    ///   not just the leading `TaskHeader` prefix. Same construction rule
    ///   as [`Self::poll_fn`]: derive `ptr` from a slot-wide reference
    ///   (e.g. `NonNull::from(&mut slot).cast::<TaskHeader>()`).
    /// * The caller must hold exclusive access to the slot — typically the
    ///   slab reclaim path or a Conductor arena reset.
    ///
    /// Branching is performed on the loaded state with `Acquire` ordering
    /// per the layout contract:
    ///
    /// * `Sleeping` / `Woken` / `Running` → drop the future (poll has not
    ///   returned `Ready`, so the output half is uninit).
    /// * `Done` → drop the output (the future was already dropped by
    ///   `poll_fn`; the join handle never consumed the value).
    /// * `Cancelled` / `Failed` / `Taken` → no drops — the live half was
    ///   already cleaned up by whichever path produced the terminal state.
    fn drop_in_place_fn(ptr: NonNull<TaskHeader>) {
        let slot: *mut Self = ptr.as_ptr().cast();
        // SAFETY: `state` is a live field of the header; provenance covers
        // the full slot so the read is in-bounds.
        let state = unsafe { (*ptr.as_ptr()).state.load() };
        match state {
            TaskState::Sleeping | TaskState::Woken | TaskState::Running => {
                // SAFETY: future half is still init (no Ready yet).
                unsafe {
                    ptr::drop_in_place(&raw mut (*slot).cell.future);
                }
            }
            TaskState::Done => {
                // SAFETY: output half was written by `poll_fn` before state
                // moved to `Done`; future was dropped at the same site.
                unsafe {
                    (*slot).cell.output.assume_init_drop();
                }
            }
            TaskState::Cancelled | TaskState::Failed | TaskState::Taken => {
                // No drops — both halves are already cleaned up:
                //  - Cancelled/Failed paths drop the future before setting state
                //  - Taken means the join handle consumed the output
            }
        }
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;

    use core::mem::offset_of;
    use core::ptr::addr_of_mut;
    use core::sync::atomic::AtomicUsize;
    use core::sync::atomic::Ordering;
    use core::task::{RawWaker, RawWakerVTable, Waker};

    /// Counts how many times each side of the cell has been dropped, used by
    /// `drop_in_place` branching tests.
    #[derive(Default)]
    struct DropCounts {
        future: AtomicUsize,
        output: AtomicUsize,
    }

    /// Future that records its drop in `counts.future` and yields a value
    /// whose drop records into `counts.output`.
    struct ProbeFuture {
        counts: &'static DropCounts,
        ready: bool,
    }

    struct ProbeOutput {
        counts: &'static DropCounts,
    }

    impl Drop for ProbeOutput {
        fn drop(&mut self) {
            self.counts.output.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl Drop for ProbeFuture {
        fn drop(&mut self) {
            self.counts.future.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl Future for ProbeFuture {
        type Output = ProbeOutput;
        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<ProbeOutput> {
            if self.ready {
                Poll::Ready(ProbeOutput {
                    counts: self.counts,
                })
            } else {
                Poll::Pending
            }
        }
    }

    const fn dummy_waker() -> Waker {
        const VTABLE: RawWakerVTable = RawWakerVTable::new(
            |_| RawWaker::new(ptr::null(), &VTABLE),
            |_| {},
            |_| {},
            |_| {},
        );
        // SAFETY: vtable pointers are no-ops that never dereference `data`.
        unsafe { Waker::from_raw(RawWaker::new(ptr::null(), &VTABLE)) }
    }

    /// Build a vtable-grade `NonNull<TaskHeader>` from a slot. Provenance
    /// covers the entire `Slot<F>` (offset 0 cast through repr(C) header
    /// prefix) per the contract on `Slot::poll_fn` / `Slot::drop_in_place_fn`.
    fn header_nn<F: Future>(slot: &mut Slot<F>) -> NonNull<TaskHeader> {
        NonNull::from(slot).cast()
    }

    #[test]
    fn output_offset_equals_header_size() {
        assert_eq!(Slot::<ProbeFuture>::OUTPUT_OFFSET, size_of::<TaskHeader>());
    }

    #[test]
    fn slot_field_order_is_repr_c() {
        let h = offset_of!(Slot<ProbeFuture>, header);
        let c = offset_of!(Slot<ProbeFuture>, cell);
        assert_eq!(h, 0, "header must be at offset 0 for type-erased cast");
        assert!(c >= size_of::<TaskHeader>(), "cell must follow header");
    }

    #[test]
    fn slot_size_under_512_bytes_const_assert() {
        let () = Slot::<ProbeFuture>::SIZE_OK;
    }

    #[test]
    fn header_field_order_unchanged() {
        let s = offset_of!(TaskHeader, state);
        let n = offset_of!(TaskHeader, nid);
        let ns = offset_of!(TaskHeader, namespace);
        let fc = offset_of!(TaskHeader, first_child);
        let nx = offset_of!(TaskHeader, next_sibling);
        let v = offset_of!(TaskHeader, vtable);
        assert!(s < n);
        assert!(n < ns);
        assert!(ns < fc);
        assert!(fc < nx);
        assert!(nx < v, "vtable must be last so prior offsets stay frozen");
    }

    #[test]
    fn poll_fn_pending_keeps_output_uninit() {
        static COUNTS: DropCounts = DropCounts {
            future: AtomicUsize::new(0),
            output: AtomicUsize::new(0),
        };
        let mut slot = Slot::new(
            Nid::detached(),
            Namespace::ROOT,
            ProbeFuture {
                counts: &COUNTS,
                ready: false,
            },
        );
        // Caller-side state transition: Sleeping -> Running.
        let Ok(()) = slot
            .header
            .state
            .transition(TaskState::Sleeping, TaskState::Running)
        else {
            panic!("Sleeping -> Running must succeed");
        };
        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);
        let poll = slot.header.vtable.poll;
        // header_nn is invoked at each vtable call site so the SharedReadWrite
        // borrow is freshly retagged; otherwise an intervening write through
        // `slot.header.state` would invalidate the prior tag under Stacked
        // Borrows.
        let result = poll(header_nn(&mut slot), &mut cx);
        assert!(matches!(result, Poll::Pending));
        // future still alive; the slot drop at end-of-test will fire it.
    }

    #[test]
    fn poll_fn_ready_writes_output_and_drops_future() {
        static COUNTS: DropCounts = DropCounts {
            future: AtomicUsize::new(0),
            output: AtomicUsize::new(0),
        };
        let mut slot = Slot::new(
            Nid::detached(),
            Namespace::ROOT,
            ProbeFuture {
                counts: &COUNTS,
                ready: true,
            },
        );
        let Ok(()) = slot
            .header
            .state
            .transition(TaskState::Sleeping, TaskState::Running)
        else {
            panic!("Sleeping -> Running must succeed");
        };
        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);
        let poll = slot.header.vtable.poll;
        let drop_in_place = slot.header.vtable.drop_in_place;
        let result = poll(header_nn(&mut slot), &mut cx);
        assert!(matches!(result, Poll::Ready(())));
        assert_eq!(COUNTS.future.load(Ordering::Relaxed), 1);
        assert_eq!(COUNTS.output.load(Ordering::Relaxed), 0);
        // Caller-side post-poll transition.
        let Ok(()) = slot
            .header
            .state
            .transition(TaskState::Running, TaskState::Done)
        else {
            panic!("Running -> Done must succeed");
        };
        // state is Done — drop_in_place_fn drops the output. A fresh NonNull
        // is needed because the Running->Done CAS above invalidates any prior
        // SharedReadWrite borrow under Stacked Borrows.
        drop_in_place(header_nn(&mut slot));
        assert_eq!(COUNTS.output.load(Ordering::Relaxed), 1);
        // Prevent the local slot's normal drop from running cell again.
        core::mem::forget(slot);
    }

    #[test]
    fn drop_in_place_pending_drops_future_only() {
        static COUNTS: DropCounts = DropCounts {
            future: AtomicUsize::new(0),
            output: AtomicUsize::new(0),
        };
        let mut slot = Slot::new(
            Nid::detached(),
            Namespace::ROOT,
            ProbeFuture {
                counts: &COUNTS,
                ready: false,
            },
        );
        let drop_in_place = slot.header.vtable.drop_in_place;
        // state is Sleeping (initial) — future half still init, output uninit.
        drop_in_place(header_nn(&mut slot));
        assert_eq!(COUNTS.future.load(Ordering::Relaxed), 1);
        assert_eq!(COUNTS.output.load(Ordering::Relaxed), 0);
        core::mem::forget(slot);
    }

    #[test]
    fn drop_in_place_taken_does_not_drop_either_half() {
        static COUNTS: DropCounts = DropCounts {
            future: AtomicUsize::new(0),
            output: AtomicUsize::new(0),
        };
        let mut slot = Slot::new(
            Nid::detached(),
            Namespace::ROOT,
            ProbeFuture {
                counts: &COUNTS,
                ready: true,
            },
        );
        // Run a poll Ready first so the future is consumed; then advance to
        // Done and finally Taken to mimic Handle::join completion.
        let Ok(()) = slot
            .header
            .state
            .transition(TaskState::Sleeping, TaskState::Running)
        else {
            panic!("Sleeping -> Running must succeed");
        };
        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);
        let poll = slot.header.vtable.poll;
        let drop_in_place = slot.header.vtable.drop_in_place;
        let _ = poll(header_nn(&mut slot), &mut cx);
        let Ok(()) = slot
            .header
            .state
            .transition(TaskState::Running, TaskState::Done)
        else {
            panic!("Running -> Done must succeed");
        };
        // Simulate Handle::join consuming the output.
        // SAFETY: state is Done; reading from output before transitioning to
        // Taken is the join sequence.
        let output = unsafe { (*addr_of_mut!(slot.cell.output)).assume_init_read() };
        let Ok(()) = slot
            .header
            .state
            .transition(TaskState::Done, TaskState::Taken)
        else {
            panic!("Done -> Taken must succeed");
        };
        // Drop here — mirrors the join handle releasing the value before the
        // slot is recycled. Without an explicit drop, the binding lives to the
        // end of the test and the output count stays at zero when asserted.
        drop(output);
        // state is Taken — both halves already spent, drop_in_place_fn no-ops.
        drop_in_place(header_nn(&mut slot));
        // Drop counts: future from poll Ready (1), output from the simulated
        // join handle reading and dropping the value (1). drop_in_place_fn
        // adds neither.
        assert_eq!(COUNTS.future.load(Ordering::Relaxed), 1);
        assert_eq!(COUNTS.output.load(Ordering::Relaxed), 1);
        core::mem::forget(slot);
    }
}

//! [`AtomicTaskState`] — atomic lifecycle state for a task with a CAS loop
//! that resolves the wake-vs-terminal TOCTOU race.
//!
//! [`TaskState`] is `repr(u8)`; the [`AtomicU8`] discriminant is the only
//! source of state truth. [`AtomicTaskState::transition`] rejects terminal
//! `expected` states without a CAS attempt and re-inspects every CAS failure
//! inside the loop body, so a concurrent terminal transition cannot be mistaken
//! for a spurious failure and retried indefinitely.
//!
//! ```text
//! Sleeping ──wake()──► Woken ──poll──► Running ──Ready──► Done ──join──► Taken (terminal)
//!                                              ↘──────────► Cancelled         (terminal)
//!                                              ↘──────────► Failed            (terminal)
//! ```
#![allow(
    dead_code,
    reason = "consumed by upcoming P2 PR4 task::header and task::waker"
)]
#![allow(
    clippy::redundant_pub_crate,
    reason = "private module — pub(crate) is preferred over pub here to satisfy the workspace `unreachable_pub` lint"
)]

#[cfg(not(loom))]
use core::sync::atomic::{AtomicU8, Ordering};

#[cfg(loom)]
use loom::sync::atomic::{AtomicU8, Ordering};

/// Lifecycle state of a task.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum TaskState {
    /// Initial state. No waker pending and the future is not running.
    Sleeping = 0,
    /// `wake()` set this state. A worker will poll the task soon.
    Woken = 1,
    /// Currently being polled on a worker.
    Running = 2,
    /// Output written to the cell, awaiting [`TaskState::Taken`] by the join
    /// handle. Not terminal — a join is expected to follow.
    Done = 3,
    /// Terminal — cancelled before completion.
    Cancelled = 4,
    /// Terminal — panicked or returned an unrecoverable error.
    Failed = 5,
    /// Terminal — the join handle has read and consumed the output.
    Taken = 6,
}

impl TaskState {
    /// Returns `true` for [`TaskState::Cancelled`], [`TaskState::Failed`],
    /// and [`TaskState::Taken`].
    ///
    /// [`TaskState::Done`] is *not* terminal: the output is sitting in the
    /// cell waiting for the join handle to consume it via the
    /// `Done → Taken` transition.
    pub(crate) const fn is_terminal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Failed | Self::Taken)
    }

    /// Recovers a [`TaskState`] from a raw `u8` discriminant.
    ///
    /// The atomic only stores values written from a [`TaskState`] cast, so
    /// the `panic!` arm is unreachable in safe code. It exists to maintain
    /// totality without resorting to `unsafe` transmute.
    const fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Sleeping,
            1 => Self::Woken,
            2 => Self::Running,
            3 => Self::Done,
            4 => Self::Cancelled,
            5 => Self::Failed,
            6 => Self::Taken,
            _ => panic!("invalid TaskState discriminant"),
        }
    }
}

/// Atomic [`TaskState`] container backed by an [`AtomicU8`].
#[derive(Debug)]
pub(crate) struct AtomicTaskState(AtomicU8);

impl AtomicTaskState {
    /// New state initialised to [`TaskState::Sleeping`].
    #[cfg(not(loom))]
    #[inline]
    pub(crate) const fn new() -> Self {
        Self(AtomicU8::new(TaskState::Sleeping as u8))
    }

    /// New state initialised to [`TaskState::Sleeping`].
    ///
    /// Non-`const` under loom because `loom::sync::atomic::AtomicU8::new` is
    /// instrumented and not available in const context.
    #[cfg(loom)]
    pub(crate) fn new() -> Self {
        Self(AtomicU8::new(TaskState::Sleeping as u8))
    }

    /// Loads the current state with `Acquire` ordering.
    #[inline]
    pub(crate) fn load(&self) -> TaskState {
        TaskState::from_u8(self.0.load(Ordering::Acquire))
    }

    /// Attempts the transition `expected → next`. Returns the actually
    /// observed state on failure.
    ///
    /// # TOCTOU
    ///
    /// The terminal check is performed *inside* the CAS-failure arm, never
    /// between a separate `load` and the CAS. A naïve
    /// `if !load().is_terminal() { CAS } else { Err }` race-loses against a
    /// concurrent terminal transition: the load reads a non-terminal value,
    /// the CAS proceeds and fails because another thread already moved on,
    /// and the loop retries forever. Inspecting the `Err(current)` from the
    /// CAS itself observes the post-write state and distinguishes "terminal"
    /// from "spurious".
    ///
    /// # Errors
    ///
    /// Returns `Err(current)` when `expected` is already terminal (early
    /// reject without a CAS) or when a concurrent transition raced ahead.
    /// The error carries the actual state observed at CAS time.
    ///
    /// # Examples
    ///
    /// ```text
    /// // Internal use within nabi-runtime.
    /// let state = AtomicTaskState::new();
    /// state.transition(TaskState::Sleeping, TaskState::Running)?;
    /// state.transition(TaskState::Running, TaskState::Done)?;
    /// // state.transition(TaskState::Done, _) returns Err(Done) — terminal.
    /// ```
    pub(crate) fn transition(&self, expected: TaskState, next: TaskState) -> Result<(), TaskState> {
        if expected.is_terminal() {
            return Err(self.load());
        }
        loop {
            match self.0.compare_exchange_weak(
                expected as u8,
                next as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(()),
                Err(current_raw) => {
                    let current = TaskState::from_u8(current_raw);
                    if current.is_terminal() || current != expected {
                        return Err(current);
                    }
                    // Spurious failure (`compare_exchange_weak` permits one
                    // even when `current == expected`): retry the same CAS.
                }
            }
        }
    }

    /// Convenience wrapper for the [`TaskState::Sleeping`] →
    /// [`TaskState::Woken`] transition.
    ///
    /// # Errors
    ///
    /// Returns the observed state if the task is no longer
    /// [`TaskState::Sleeping`].
    #[inline]
    pub(crate) fn wake(&self) -> Result<(), TaskState> {
        self.transition(TaskState::Sleeping, TaskState::Woken)
    }
}

impl Default for AtomicTaskState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, not(loom)))]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_sleeping() {
        let state = AtomicTaskState::new();
        assert_eq!(state.load(), TaskState::Sleeping);
    }

    #[test]
    fn wake_from_sleeping_succeeds_and_sets_woken() {
        let state = AtomicTaskState::new();
        let Ok(()) = state.wake() else {
            panic!("wake from Sleeping must succeed");
        };
        assert_eq!(state.load(), TaskState::Woken);
    }

    #[test]
    fn wake_from_woken_returns_woken() {
        let state = AtomicTaskState::new();
        let Ok(()) = state.wake() else {
            panic!("first wake must succeed");
        };
        match state.wake() {
            Err(TaskState::Woken) => {}
            other => panic!("second wake should observe Woken, got {other:?}"),
        }
    }

    #[test]
    fn lifecycle_sleeping_woken_running_done() {
        let state = AtomicTaskState::new();
        let Ok(()) = state.wake() else {
            panic!("Sleeping -> Woken must succeed");
        };
        let Ok(()) = state.transition(TaskState::Woken, TaskState::Running) else {
            panic!("Woken -> Running must succeed");
        };
        let Ok(()) = state.transition(TaskState::Running, TaskState::Done) else {
            panic!("Running -> Done must succeed");
        };
        assert_eq!(state.load(), TaskState::Done);
    }

    #[test]
    fn running_to_cancelled_succeeds() {
        let state = AtomicTaskState::new();
        let Ok(()) = state.transition(TaskState::Sleeping, TaskState::Running) else {
            panic!("Sleeping -> Running must succeed");
        };
        let Ok(()) = state.transition(TaskState::Running, TaskState::Cancelled) else {
            panic!("Running -> Cancelled must succeed");
        };
        assert_eq!(state.load(), TaskState::Cancelled);
    }

    #[test]
    fn running_to_failed_succeeds() {
        let state = AtomicTaskState::new();
        let Ok(()) = state.transition(TaskState::Sleeping, TaskState::Running) else {
            panic!("Sleeping -> Running must succeed");
        };
        let Ok(()) = state.transition(TaskState::Running, TaskState::Failed) else {
            panic!("Running -> Failed must succeed");
        };
        assert_eq!(state.load(), TaskState::Failed);
    }

    #[test]
    fn terminal_rejects_wake_and_transition() {
        let state = AtomicTaskState::new();
        let Ok(()) = state.transition(TaskState::Sleeping, TaskState::Cancelled) else {
            panic!("Sleeping -> Cancelled must succeed");
        };
        match state.wake() {
            Err(TaskState::Cancelled) => {}
            other => panic!("wake on Cancelled must fail, got {other:?}"),
        }
        match state.transition(TaskState::Cancelled, TaskState::Sleeping) {
            Err(TaskState::Cancelled) => {}
            other => panic!("Cancelled is terminal, expected Err(Cancelled) got {other:?}"),
        }
    }

    #[test]
    fn done_to_taken_succeeds() {
        let state = AtomicTaskState::new();
        let Ok(()) = state.transition(TaskState::Sleeping, TaskState::Running) else {
            panic!("Sleeping -> Running must succeed");
        };
        let Ok(()) = state.transition(TaskState::Running, TaskState::Done) else {
            panic!("Running -> Done must succeed");
        };
        let Ok(()) = state.transition(TaskState::Done, TaskState::Taken) else {
            panic!("Done -> Taken must succeed (output consumed by join handle)");
        };
        assert_eq!(state.load(), TaskState::Taken);
    }

    #[test]
    fn taken_rejects_further_transition() {
        let state = AtomicTaskState::new();
        let Ok(()) = state.transition(TaskState::Sleeping, TaskState::Taken) else {
            panic!("Sleeping -> Taken must succeed");
        };
        match state.transition(TaskState::Taken, TaskState::Sleeping) {
            Err(TaskState::Taken) => {}
            other => panic!("Taken is terminal, expected Err(Taken) got {other:?}"),
        }
    }

    #[test]
    fn is_terminal_classifies_every_variant() {
        assert!(!TaskState::Sleeping.is_terminal());
        assert!(!TaskState::Woken.is_terminal());
        assert!(!TaskState::Running.is_terminal());
        assert!(!TaskState::Done.is_terminal());
        assert!(TaskState::Cancelled.is_terminal());
        assert!(TaskState::Failed.is_terminal());
        assert!(TaskState::Taken.is_terminal());
    }

    #[test]
    fn from_u8_round_trips_every_variant() {
        for variant in [
            TaskState::Sleeping,
            TaskState::Woken,
            TaskState::Running,
            TaskState::Done,
            TaskState::Cancelled,
            TaskState::Failed,
            TaskState::Taken,
        ] {
            assert_eq!(TaskState::from_u8(variant as u8), variant);
        }
    }
}

#[cfg(all(test, loom))]
mod loom_tests {
    use super::*;

    use loom::sync::Arc;
    use loom::thread;

    #[test]
    fn dual_woken_to_running_only_one_wins() {
        loom::model(|| {
            let state = Arc::new(AtomicTaskState::new());
            let Ok(()) = state.wake() else {
                panic!("Sleeping -> Woken must succeed");
            };
            let s1 = Arc::clone(&state);
            let s2 = Arc::clone(&state);
            let h1 = thread::spawn(move || s1.transition(TaskState::Woken, TaskState::Running));
            let h2 = thread::spawn(move || s2.transition(TaskState::Woken, TaskState::Running));
            let Ok(r1) = h1.join() else {
                panic!("h1 panicked");
            };
            let Ok(r2) = h2.join() else {
                panic!("h2 panicked");
            };
            let wins = usize::from(r1.is_ok()) + usize::from(r2.is_ok());
            assert_eq!(wins, 1, "exactly one Woken -> Running must succeed");
            assert_eq!(state.load(), TaskState::Running);
        });
    }

    #[test]
    fn running_to_done_vs_cancelled_only_one_wins() {
        loom::model(|| {
            let state = Arc::new(AtomicTaskState::new());
            let Ok(()) = state.transition(TaskState::Sleeping, TaskState::Running) else {
                panic!("Sleeping -> Running must succeed");
            };
            let s1 = Arc::clone(&state);
            let s2 = Arc::clone(&state);
            let h1 = thread::spawn(move || s1.transition(TaskState::Running, TaskState::Done));
            let h2 = thread::spawn(move || s2.transition(TaskState::Running, TaskState::Cancelled));
            let Ok(r1) = h1.join() else {
                panic!("h1 panicked");
            };
            let Ok(r2) = h2.join() else {
                panic!("h2 panicked");
            };
            match (r1, r2, state.load()) {
                (Ok(()), Err(TaskState::Done), TaskState::Done) => {}
                (Err(TaskState::Cancelled), Ok(()), TaskState::Cancelled) => {}
                other => panic!("unexpected race outcome: {other:?}"),
            }
        });
    }

    #[test]
    fn wake_and_transition_woken_running_observe_consistent_state() {
        loom::model(|| {
            let state = Arc::new(AtomicTaskState::new());
            let s_wake = Arc::clone(&state);
            let s_run = Arc::clone(&state);
            let h_wake = thread::spawn(move || s_wake.wake());
            let h_run =
                thread::spawn(move || s_run.transition(TaskState::Woken, TaskState::Running));
            let Ok(r_wake) = h_wake.join() else {
                panic!("wake thread panicked");
            };
            let Ok(r_run) = h_run.join() else {
                panic!("transition thread panicked");
            };
            let Ok(()) = r_wake else {
                panic!("wake from Sleeping must succeed: {r_wake:?}");
            };
            match r_run {
                Ok(()) => assert_eq!(state.load(), TaskState::Running),
                Err(TaskState::Sleeping) => assert_eq!(state.load(), TaskState::Woken),
                other => panic!("unexpected r_run: {other:?}"),
            }
        });
    }
}

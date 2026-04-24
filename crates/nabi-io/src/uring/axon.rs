//! Minimal `io_uring` axon — environment probe implementation.
//!
//! Bootstraps a fixed-size (32-entry) ring and submits a single
//! `IORING_OP_NOP` to verify SQ/CQ wiring end-to-end. This is the seed
//! for the full `Axon` trait implementation (multishot, buffer
//! registration, `msg_ring`, cancellation) — the interface is *not*
//! stable across that boundary and will be renamed once the trait lands.

use std::io;

use io_uring::{IoUring, opcode};

/// Fixed SQ/CQ depth for the probe.
const PROBE_RING_ENTRIES: u32 = 32;

/// Owning handle to an `io_uring` instance.
///
/// `IoUring::drop` performs `munmap` of the SQ/CQ/SQE regions and
/// `close` of the ring fd, so no manual teardown is required here.
pub struct UringAxon {
    ring: IoUring,
}

impl UringAxon {
    /// Bootstrap a ring with 32 slots in both SQ and CQ.
    ///
    /// # Errors
    ///
    /// Returns the underlying `io::Error` from `io_uring_setup(2)` if the
    /// kernel refuses ring creation (e.g. `kernel.io_uring_disabled = 1`,
    /// `RLIMIT_MEMLOCK` exhausted, or a kernel older than 5.1).
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            ring: IoUring::new(PROBE_RING_ENTRIES)?,
        })
    }

    /// Submit a single `IORING_OP_NOP` carrying `user_data`, wait for its
    /// completion, and return `(user_data, result, flags)` from the CQE.
    ///
    /// # Errors
    ///
    /// Returns an `io::Error` if:
    /// - the SQE cannot be pushed to a full submission queue,
    /// - `io_uring_enter(2)` fails during submit,
    /// - no CQE is available after `submit_and_wait(1)` returns (should
    ///   not happen in practice and indicates a kernel-side invariant
    ///   violation).
    pub fn nop_probe(&mut self, user_data: u64) -> io::Result<(u64, i32, u32)> {
        let entry = opcode::Nop::new().build().user_data(user_data);

        // SAFETY: `entry` is a well-formed Nop SQE with no buffer, fd, or
        // msghdr references; nothing the SQE points to must outlive this
        // call. The submission queue is accessed exclusively via
        // `&mut self`, so no concurrent producer exists.
        unsafe {
            self.ring
                .submission()
                .push(&entry)
                .map_err(io::Error::other)?;
        }

        self.ring.submit_and_wait(1)?;

        let cqe = self
            .ring
            .completion()
            .next()
            .ok_or_else(|| io::Error::other("no CQE after submit_and_wait(1)"))?;

        Ok((cqe.user_data(), cqe.result(), cqe.flags()))
    }
}

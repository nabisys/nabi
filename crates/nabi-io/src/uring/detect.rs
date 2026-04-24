//! Kernel `io_uring` feature probe.
//!
//! Bootstraps a minimal ring (8 entries) and reads the returned
//! `io_uring::Parameters` to classify which features the running kernel
//! supports. Used by [`super::axon`] to pick code paths and by tests to
//! guard kernel-version-specific assumptions.

use std::io;

use io_uring::IoUring;

/// Features reported by the kernel at ring initialization.
///
/// Populated from `io_uring_setup`'s returned `params.features` bitmask
/// via [`UringCapabilities::detect`]. This set is intentionally minimal;
/// additional flags (`SINGLE_ISSUER`, `DEFER_TASKRUN`, `COOP_TASKRUN`,
/// ...) are introduced alongside the full axon layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UringCapabilities {
    /// `IORING_FEAT_NODROP` — CQ overflow is reported via
    /// `IORING_SQ_CQ_OVERFLOW` instead of silently dropping CQEs.
    /// Present since Linux 5.1.
    pub nodrop: bool,
}

impl UringCapabilities {
    /// Probe the running kernel by bootstrapping a throwaway ring and
    /// inspecting the returned feature bits.
    ///
    /// # Errors
    ///
    /// Returns the underlying `io::Error` from `io_uring_setup(2)` if the
    /// kernel refuses ring creation — for example `kernel.io_uring_disabled = 1`
    /// is set, the process lacks `CAP_SYS_ADMIN` where required, or the
    /// running kernel predates `io_uring` entirely (< 5.1).
    pub fn detect() -> io::Result<Self> {
        let ring = IoUring::new(8)?;
        let params = ring.params();
        Ok(Self {
            nodrop: params.is_feature_nodrop(),
        })
    }
}

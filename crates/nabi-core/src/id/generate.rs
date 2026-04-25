//! [`Nid`] constructors — `root`, `root_on`, `child`, `detached`.

use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    Nid,
    error::NidError,
    layout::{CURRENT_VERSION, DEPTH_SHIFT, SEQ_SHIFT, VERSION_SHIFT, WORKER_BITS, WORKER_SHIFT},
};

/// Process-wide monotonic sequence counter. Starts at 1 (0 reserved for "no id").
static GLOBAL_SEQ: AtomicU64 = AtomicU64::new(1);

#[inline]
fn next_seq() -> u64 {
    GLOBAL_SEQ.fetch_add(1, Ordering::Relaxed)
}

/// Pack the bit fields into a raw `u128`. `kind` and `flags` are 0 within P0.
#[inline]
const fn pack(seq: u64, depth: u16, worker: u64) -> u128 {
    let worker_masked = (worker as u128) & ((1u128 << WORKER_BITS) - 1);
    ((seq as u128) << SEQ_SHIFT)
        | ((depth as u128) << DEPTH_SHIFT)
        | ((CURRENT_VERSION as u128) << VERSION_SHIFT)
        | (worker_masked << WORKER_SHIFT)
}

impl Nid {
    /// Create a root-level `Nid` with depth 0 and no worker hint.
    #[inline]
    pub fn root() -> Self {
        Self(pack(next_seq(), 0, 0))
    }

    /// Create a root-level `Nid` pinned to a specific worker.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if `worker_id` exceeds `WORKER_BITS` (38 bits).
    /// In release builds the value is silently masked.
    #[inline]
    pub fn root_on(worker_id: u64) -> Self {
        debug_assert!(
            worker_id < (1u64 << WORKER_BITS),
            "worker_id exceeds WORKER_BITS",
        );
        Self(pack(next_seq(), 0, worker_id))
    }

    /// Create a child `Nid` nested one level below `self`.
    ///
    /// Inherits the parent's worker hint, increments depth by 1, and assigns
    /// a fresh sequence number.
    ///
    /// # Errors
    ///
    /// Returns [`NidError::DepthOverflow`] if the depth counter would exceed
    /// `u16::MAX`.
    #[inline]
    pub fn child(self) -> Result<Self, NidError> {
        let child_depth = self.depth().checked_add(1).ok_or(NidError::DepthOverflow)?;
        Ok(Self(pack(next_seq(), child_depth, self.worker_id())))
    }

    /// Create a detached `Nid` with no parent relationship.
    ///
    /// Equivalent in effect to [`Self::root`]; the distinct name documents
    /// intent at the call site (e.g., a task spawned outside any conductor).
    #[inline]
    pub fn detached() -> Self {
        Self::root()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_has_depth_zero_no_worker() {
        let id = Nid::root();
        assert_eq!(id.depth(), 0);
        assert_eq!(id.worker_id(), 0);
    }

    #[test]
    fn root_ids_have_unique_increasing_seq() {
        let a = Nid::root();
        let b = Nid::root();
        assert!(b.seq() > a.seq());
    }

    #[test]
    fn root_on_pins_worker() {
        let id = Nid::root_on(42);
        assert_eq!(id.worker_id(), 42);
        assert_eq!(id.depth(), 0);
    }

    #[test]
    #[cfg_attr(debug_assertions, should_panic(expected = "worker_id exceeds"))]
    fn root_on_panics_on_worker_overflow_in_debug() {
        let _ = Nid::root_on(1u64 << WORKER_BITS);
    }

    #[test]
    fn child_increments_depth() {
        let root = Nid::root();
        let Ok(child) = root.child() else {
            unreachable!("root has depth 0, child cannot overflow")
        };
        assert_eq!(child.depth(), 1);
    }

    #[test]
    fn child_inherits_worker() {
        let root = Nid::root_on(7);
        let Ok(child) = root.child() else {
            unreachable!("root has depth 0, child cannot overflow")
        };
        assert_eq!(child.worker_id(), 7);
    }

    #[test]
    fn child_depth_overflow_at_u16_max() {
        let raw = (1u128 << SEQ_SHIFT) | (u128::from(u16::MAX) << DEPTH_SHIFT);
        let maxed = Nid(raw);
        assert!(matches!(maxed.child(), Err(NidError::DepthOverflow)));
    }

    #[test]
    fn detached_is_root_equivalent() {
        let d = Nid::detached();
        assert_eq!(d.depth(), 0);
        assert_eq!(d.worker_id(), 0);
    }
}

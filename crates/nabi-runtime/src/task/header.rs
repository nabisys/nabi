//! [`TaskHeader`] — repr(C) per-task control block.
//!
//! `repr(C)` pins field order so a fixed-offset trailing payload can be added
//! later without disturbing existing fields.
#![allow(
    clippy::redundant_pub_crate,
    reason = "satisfies the workspace `unreachable_pub` lint on a private module"
)]
#![allow(dead_code, reason = "no non-test caller in this revision")]

use nabi_core::id::Nid;
use nabi_core::namespace::Namespace;

use crate::task::TaskRef;
use crate::task::state::AtomicTaskState;

/// Per-task control block.
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
///
/// # Examples
///
/// ```text
/// // Internal use within nabi-runtime.
/// let header = TaskHeader::new(Nid::detached(), Namespace::ROOT);
/// assert_eq!(header.state.load(), TaskState::Sleeping);
/// assert!(header.first_child.is_none());
/// ```
#[derive(Debug)]
#[repr(C)]
pub(crate) struct TaskHeader {
    pub(crate) state: AtomicTaskState,
    pub(crate) nid: Nid,
    pub(crate) namespace: Namespace,
    pub(crate) first_child: Option<TaskRef>,
    pub(crate) next_sibling: Option<TaskRef>,
}

impl TaskHeader {
    /// Constructs a new header in `Sleeping` with no children.
    ///
    /// `nid` and `namespace` are caller-supplied; everything else is wired to
    /// the canonical "fresh task" defaults.
    #[cfg(not(loom))]
    #[inline]
    pub(crate) const fn new(nid: Nid, namespace: Namespace) -> Self {
        Self {
            state: AtomicTaskState::new(),
            nid,
            namespace,
            first_child: None,
            next_sibling: None,
        }
    }

    /// Loom variant — `loom::sync::atomic::AtomicU8::new` is not available in
    /// const context, so the const qualifier is dropped under `--cfg loom`.
    #[cfg(loom)]
    #[inline]
    pub(crate) fn new(nid: Nid, namespace: Namespace) -> Self {
        Self {
            state: AtomicTaskState::new(),
            nid,
            namespace,
            first_child: None,
            next_sibling: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use core::mem::offset_of;

    use crate::task::state::TaskState;

    #[test]
    fn new_initialises_state_sleeping_and_no_children() {
        let header = TaskHeader::new(Nid::detached(), Namespace::ROOT);
        assert_eq!(header.state.load(), TaskState::Sleeping);
        assert!(header.first_child.is_none());
        assert!(header.next_sibling.is_none());
        assert_eq!(header.namespace, Namespace::ROOT);
    }

    #[test]
    fn new_preserves_caller_supplied_identity() {
        let nid = Nid::root_on(7);
        let ns = Namespace(42);
        let header = TaskHeader::new(nid, ns);
        assert_eq!(header.nid, nid);
        assert_eq!(header.namespace, ns);
    }

    #[test]
    fn repr_c_field_order_is_declaration_order() {
        let s = offset_of!(TaskHeader, state);
        let n = offset_of!(TaskHeader, nid);
        let ns = offset_of!(TaskHeader, namespace);
        let fc = offset_of!(TaskHeader, first_child);
        let nx = offset_of!(TaskHeader, next_sibling);
        assert!(s < n, "state must precede nid");
        assert!(n < ns, "nid must precede namespace");
        assert!(ns < fc, "namespace must precede first_child");
        assert!(fc < nx, "first_child must precede next_sibling");
    }

    #[test]
    fn option_task_ref_is_copy() {
        const fn requires_copy<T: Copy>() {}
        requires_copy::<Option<TaskRef>>();
    }
}

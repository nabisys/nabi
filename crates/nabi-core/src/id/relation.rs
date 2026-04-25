//! Heuristic ancestry checks for [`Nid`].
//!
//! These predicates compare depth and sequence numbers and do **not** verify
//! actual structural parentage. Two unrelated ids created in the right order
//! can satisfy these predicates. Use only as a fast filter, never as a
//! security or correctness boundary.

use super::Nid;

impl Nid {
    /// Returns `true` if `self` could be a direct parent of `other`.
    ///
    /// Heuristic: `self.depth() + 1 == other.depth()` and `self.seq() < other.seq()`.
    #[inline]
    pub const fn is_parent_of(self, other: Self) -> bool {
        self.depth().wrapping_add(1) == other.depth() && self.seq() < other.seq()
    }

    /// Returns `true` if `self` could be an ancestor (any depth) of `other`.
    ///
    /// Heuristic: `self.depth() < other.depth()` and `self.seq() < other.seq()`.
    #[inline]
    pub const fn is_ancestor_of(self, other: Self) -> bool {
        self.depth() < other.depth() && self.seq() < other.seq()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_parent_of_direct_child() {
        let root = Nid::root();
        let child = root.child().expect("child must not overflow");
        assert!(root.is_parent_of(child));
    }

    #[test]
    fn is_parent_of_rejects_self() {
        let root = Nid::root();
        assert!(!root.is_parent_of(root));
    }

    #[test]
    fn is_parent_of_rejects_grandchild() {
        let root = Nid::root();
        let child = root.child().expect("child must not overflow");
        let grandchild = child.child().expect("child must not overflow");
        assert!(!root.is_parent_of(grandchild));
    }

    #[test]
    fn is_parent_of_rejects_sibling() {
        let root = Nid::root();
        let a = root.child().expect("child must not overflow");
        let b = root.child().expect("child must not overflow");
        assert!(!a.is_parent_of(b));
    }

    #[test]
    fn is_ancestor_of_chain() {
        let root = Nid::root();
        let child = root.child().expect("child must not overflow");
        let grandchild = child.child().expect("child must not overflow");
        assert!(root.is_ancestor_of(child));
        assert!(root.is_ancestor_of(grandchild));
        assert!(child.is_ancestor_of(grandchild));
    }

    #[test]
    fn is_ancestor_of_rejects_self() {
        let root = Nid::root();
        assert!(!root.is_ancestor_of(root));
    }

    #[test]
    fn is_ancestor_of_rejects_sibling() {
        let root = Nid::root();
        let a = root.child().expect("child must not overflow");
        let b = root.child().expect("child must not overflow");
        assert!(!a.is_ancestor_of(b));
        assert!(!b.is_ancestor_of(a));
    }
}

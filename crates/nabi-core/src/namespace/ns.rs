/// Logical task scope.
///
/// Internally a `u32` interning index. The mapping from human-readable name
/// to `u32` is managed by a cold-path interning pool, added when use cases
/// emerge; this type itself carries no heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Namespace(pub u32);

impl Namespace {
    /// The reserved root namespace (interning pool index 0).
    pub const ROOT: Self = Self(0);

    /// Returns the raw interning index.
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl core::fmt::Display for Namespace {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ns:{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_is_zero() {
        assert_eq!(Namespace::ROOT.as_u32(), 0);
    }

    #[test]
    fn as_u32_roundtrip() {
        assert_eq!(Namespace(42).as_u32(), 42);
    }

    #[test]
    fn display_format() {
        assert_eq!(Namespace(7).to_string(), "ns:7");
        assert_eq!(Namespace::ROOT.to_string(), "ns:0");
    }

    #[test]
    fn ordering() {
        assert!(Namespace(1) > Namespace::ROOT);
        assert!(Namespace::ROOT < Namespace(1));
    }
}

//! Error type for the `id` module.

use core::fmt;

/// Errors emitted by [`super::Nid`] operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NidError {
    /// `child()` was called on a `Nid` whose depth is already at `u16::MAX`.
    DepthOverflow,
}

impl fmt::Display for NidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DepthOverflow => f.write_str("Nid depth would exceed u16::MAX"),
        }
    }
}

impl core::error::Error for NidError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_format() {
        assert_eq!(
            NidError::DepthOverflow.to_string(),
            "Nid depth would exceed u16::MAX",
        );
    }
}

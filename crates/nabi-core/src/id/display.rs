//! [`Display`] and [`Debug`] for [`Nid`].

use core::fmt;

use super::Nid;

impl fmt::Display for Nid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "nid:{:032x}", self.0)
    }
}

impl fmt::Debug for Nid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Nid")
            .field("seq", &self.seq())
            .field("depth", &self.depth())
            .field("worker", &self.worker_id())
            .field("raw", &format_args!("0x{:032x}", self.0))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_zero() {
        assert_eq!(Nid(0).to_string(), "nid:00000000000000000000000000000000",);
    }

    #[test]
    fn display_known_pattern() {
        let id = Nid(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210);
        assert_eq!(id.to_string(), "nid:0123456789abcdeffedcba9876543210");
    }

    #[test]
    fn debug_includes_field_names() {
        let dbg = format!("{:?}", Nid::root_on(7));
        assert!(dbg.contains("Nid"));
        assert!(dbg.contains("seq"));
        assert!(dbg.contains("depth"));
        assert!(dbg.contains("worker"));
        assert!(dbg.contains("raw"));
    }
}

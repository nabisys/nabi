//! `io_uring` environment probe — single E2E integration test.
//!
//! 1. `io_uring_queue_init` succeeds.
//! 2. `IORING_FEAT_NODROP` is detected (expected on any Linux >= 5.1).
//! 3. `IORING_OP_NOP` submit + CQE drain round-trip preserves `user_data`.
//! 4. Clean teardown via `Drop` (implicit at end of scope).

#![allow(
    unused_crate_dependencies,
    reason = "integration test binary inherits all dev-deps; the uring module \
              itself is cfg-gated to linux, so every target (including miri) \
              needs the lint suppressed at the crate root"
)]

#[cfg(target_os = "linux")]
mod linux {
    use nabi_io::uring::{axon::UringAxon, detect::UringCapabilities};

    /// Arbitrary 64-bit tag threaded through the NOP SQE and checked on the
    /// CQE side to verify `user_data` preservation across the
    /// submission/completion boundary.
    const MAGIC: u64 = 0xDEAD_BEEF_CAFE_BABE;

    #[cfg_attr(
        miri,
        ignore = "io_uring_setup(2) is unsupported under miri; real kernel required"
    )]
    #[test]
    fn uring_probe_queue_init_nodrop_detect_and_nop_roundtrip() {
        // 1 & 2. bootstrap + feature probe
        let caps = match UringCapabilities::detect() {
            Ok(c) => c,
            Err(e) => panic!("io_uring_queue_init failed: {e}"),
        };
        eprintln!("UringCapabilities: {caps:?}");
        assert!(
            caps.nodrop,
            "IORING_FEAT_NODROP required; kernel must be Linux >= 5.1",
        );

        // 3. NOP op E2E
        let mut axon = match UringAxon::new() {
            Ok(a) => a,
            Err(e) => panic!("ring bootstrap failed: {e}"),
        };
        let (ud, res, flags) = match axon.nop_probe(MAGIC) {
            Ok(v) => v,
            Err(e) => panic!("NOP submit/wait failed: {e}"),
        };

        assert_eq!(ud, MAGIC, "CQE user_data mismatch");
        assert_eq!(res, 0, "IORING_OP_NOP must return 0 on success");
        let _ = flags; // IORING_CQE_F_* — no flags expected for bare NOP

        // 4. `axon` drops here → IoUring::drop unmaps SQ/CQ/SQE and closes fd.
    }
}

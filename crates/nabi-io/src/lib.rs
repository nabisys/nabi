//! Platform I/O for Nabi: `io_uring`, IOCP, epoll, kqueue.
#![allow(unused_crate_dependencies, reason = "scaffolding")]

#[cfg(target_os = "linux")]
pub mod uring;

//! Phantom-type [`Mode`] markers — `Affine` (`!Send`) vs `Stealing` (`Send + Sync`).
//!
//! `TaskHandle<T, M: Mode>` (later P2 PR) parameterises over the mode to encode
//! at the type level whether a task may cross worker boundaries. The trait is
//! sealed so external crates cannot extend the axis.

use core::marker::PhantomData;

mod sealed {
    pub trait Sealed {}
}

/// `!Send` marker — task is pinned to the worker that spawned it.
///
/// The raw-pointer `PhantomData` blocks the auto-derived `Send` impl, which is
/// the desired behaviour: `Affine` tasks must never migrate.
///
/// # Examples
///
/// `Affine` deliberately does not implement `Send`:
///
/// ```compile_fail
/// use nabi_runtime::task::Affine;
/// fn assert_send<T: Send>() {}
/// assert_send::<Affine>();
/// ```
pub struct Affine(PhantomData<*const ()>);

/// `Send + Sync` marker — task may be stolen by another worker.
///
/// # Examples
///
/// ```
/// use nabi_runtime::task::Stealing;
/// fn assert_send_sync<T: Send + Sync>() {}
/// assert_send_sync::<Stealing>();
/// ```
pub struct Stealing(PhantomData<()>);

/// Sealed marker trait selecting between [`Affine`] and [`Stealing`].
///
/// External crates cannot add a third axis; the only inhabitants are the two
/// markers defined alongside this trait.
pub trait Mode: sealed::Sealed {}

impl sealed::Sealed for Affine {}
impl Mode for Affine {}

impl sealed::Sealed for Stealing {}
impl Mode for Stealing {}

const _: fn() = || {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Stealing>();
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_are_zero_sized() {
        assert_eq!(core::mem::size_of::<Affine>(), 0);
        assert_eq!(core::mem::size_of::<Stealing>(), 0);
    }
}

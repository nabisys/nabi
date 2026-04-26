/// Marker trait for types with stable, `repr(C)` byte layout.
///
/// Used for distributed zero-copy: values implementing `FlatLayout` can be
/// transmitted across nodes by treating their in-memory representation as
/// the wire format.
///
/// # Safety
///
/// Implementors must guarantee:
///
/// - `SIZE == core::mem::size_of::<Self>()`
/// - `ALIGN == core::mem::align_of::<Self>()`
/// - The type has stable layout — either a primitive or `#[repr(C)]`
pub unsafe trait FlatLayout {
    /// Size in bytes.
    const SIZE: usize;
    /// Alignment in bytes.
    const ALIGN: usize;
}

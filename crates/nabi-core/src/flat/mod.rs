//! Marker trait for types with stable, `repr(C)` byte layout, used for
//! distributed zero-copy.

mod layout;
mod primitive;

pub use layout::FlatLayout;

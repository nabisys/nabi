//! `FlatLayout` impls for primitive types.
//!
//! `usize` and `isize` are intentionally excluded — their size is
//! platform-dependent and using them in a wire format breaks
//! cross-platform compatibility. Use a fixed-size integer instead.
//!
//! `bool`, `char`, `u128`, `i128`, `Option<T>`, and tuples are also excluded.
//! `bool` and `char` have invalid bit patterns; the others may be added in a
//! later phase if call sites emerge.

use core::mem::{align_of, size_of};

use super::layout::FlatLayout;

macro_rules! impl_flat_primitive {
    ($($t:ty),* $(,)?) => {
        $(
            // SAFETY: `$t` is a Rust primitive with stable layout; SIZE and
            // ALIGN are derived from `size_of` / `align_of` of `Self`.
            unsafe impl FlatLayout for $t {
                const SIZE: usize = size_of::<Self>();
                const ALIGN: usize = align_of::<Self>();
            }
        )*
    };
}

impl_flat_primitive!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);

// SAFETY: an array `[T; N]` of `FlatLayout` elements has size `T::SIZE * N`
// and alignment `T::ALIGN`. Both are guaranteed by Rust's array layout rules.
unsafe impl<T: FlatLayout, const N: usize> FlatLayout for [T; N] {
    const SIZE: usize = T::SIZE * N;
    const ALIGN: usize = T::ALIGN;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn array_size_is_n_times_t() {
        assert_eq!(<[u32; 4] as FlatLayout>::SIZE, 16);
        assert_eq!(<[u32; 4] as FlatLayout>::ALIGN, 4);
    }

    #[test]
    fn nested_array_size() {
        assert_eq!(<[[u8; 3]; 2] as FlatLayout>::SIZE, 6);
        assert_eq!(<[[u8; 3]; 2] as FlatLayout>::ALIGN, 1);
    }

    #[test]
    fn zero_size_array() {
        assert_eq!(<[u8; 0] as FlatLayout>::SIZE, 0);
        assert_eq!(<[u8; 0] as FlatLayout>::ALIGN, 1);
    }
}

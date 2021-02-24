//! This code is inspired by (and partially borrowed from)
//! https://docs.rs/endiannezz/0.5.2/endiannezz/trait.Primitive.html
//! but there was a lot in that crate I did not want, the name did not inspire
//! confidence, and I wanted a different return value, so I just took the code
//! to modify slightly.

// TODO: figure out these macros and let us replace (self: Self) with self
#![allow(clippy::needless_arbitrary_self_type)]

use std::mem;

pub trait Endian: Sized + Copy {
    type Buf: AsRef<[u8]> + AsMut<[u8]> + Into<Vec<u8>> + Default;

    fn to_le_bytes(self) -> Self::Buf;
    fn to_be_bytes(self) -> Self::Buf;

    fn from_le_bytes(bytes: Self::Buf) -> Self;
    fn from_be_bytes(bytes: Self::Buf) -> Self;
}

macro_rules! delegate {
    ($ty:ty, [$($method:ident),* $(,)?], ($param:ident : $param_ty:ty) -> $ret:ty) => {
        delegate!(@inner $ty, [$($method),*], $param, $param_ty, $ret);
    };
    (@inner $ty:ty, [$($method:ident),*], $param:ident, $param_ty:ty, $ret:ty) => {
        $(
            #[inline]
            fn $method ($param: $param_ty) -> $ret { <$ty>::$method($param) }
        )*
    };
}

macro_rules! impl_primitives {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Endian for $ty {
                type Buf = [u8; mem::size_of::<$ty>()];

                delegate!($ty, [
                    to_le_bytes,
                    to_be_bytes,
                ], (self: Self) -> Self::Buf);

                delegate!($ty, [
                    from_le_bytes,
                    from_be_bytes,
                ], (bytes: Self::Buf) -> Self);
            }
        )*
    };
}

#[rustfmt::skip]
impl_primitives![
    i8, i16, i32, i64, i128,
    u8, u16, u32, u64, u128,
];

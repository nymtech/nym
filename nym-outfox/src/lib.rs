use std::iter::repeat_with;

pub mod error;
pub mod format;
pub mod lion;
pub mod packet;

pub fn randombytes(n: usize) -> Vec<u8> {
    repeat_with(|| fastrand::u8(..)).take(n).collect()
}

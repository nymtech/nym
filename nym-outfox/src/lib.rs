// MAX: temp ignore deprecated, can be dealt with in its own PR
#![allow(deprecated)] // silences clippy warning: deprecated associated function `chacha20::cipher::generic_array::GenericArray::<T, N>::from_slice`: please upgrade to generic-array 1.x - TODO

pub mod constants;
pub mod error;
pub mod format;
pub mod lion;
pub mod packet;

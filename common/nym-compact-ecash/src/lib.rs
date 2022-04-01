use std::convert::TryInto;

use bls12_381::Scalar;

pub use traits::Base58;

use crate::error::CompactEcashError;
use crate::traits::Bytable;

mod error;
mod proofs;
mod scheme;
#[cfg(test)]
mod tests;
mod traits;
mod utils;

pub type Attribute = Scalar;

impl Bytable for Attribute {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CompactEcashError> {
        Ok(Attribute::from_bytes(slice.try_into().unwrap()).unwrap())
    }
}

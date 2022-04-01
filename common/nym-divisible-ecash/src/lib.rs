use bls12_381::Scalar;

mod proofs;
mod scheme;
#[cfg(test)]
mod tests;
mod error;
mod utils;
mod traits;

pub type Attribute = Scalar;
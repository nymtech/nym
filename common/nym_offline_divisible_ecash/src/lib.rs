use bls12_381::Scalar;

mod error;
mod proofs;
mod scheme;
#[cfg(test)]
mod tests;
mod traits;
mod utils;
mod constants;

pub type Attribute = Scalar;

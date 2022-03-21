use bls12_381::G1Projective;

pub mod aggregation;
pub mod withdrawal;
pub mod keygen;
pub mod setup;
pub mod spend;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct BlindedSignature(G1Projective, G1Projective);
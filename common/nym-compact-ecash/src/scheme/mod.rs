use bls12_381::G1Projective;

pub mod aggregation;
pub mod keygen;
pub mod setup;
pub mod spend;
pub mod withdrawal;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct BlindedSignature(G1Projective, G1Projective);

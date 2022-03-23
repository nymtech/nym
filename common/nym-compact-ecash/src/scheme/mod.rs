use bls12_381::{G1Projective, Scalar};

pub mod aggregation;
pub mod keygen;
pub mod setup;
pub mod spend;
pub mod withdrawal;

pub type SignerIndex = u64;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Signature(pub(crate) G1Projective, pub(crate) G1Projective);

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct BlindedSignature(G1Projective, G1Projective);

pub struct Wallet {
    sig: Signature,
    v: Scalar,
    idx: Option<SignerIndex>,
}
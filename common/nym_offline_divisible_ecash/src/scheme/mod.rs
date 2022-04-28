use bls12_381::Scalar;

use crate::utils::{Signature, SignerIndex};

pub mod aggregation;
pub mod identify;
pub mod keygen;
pub mod setup;
pub mod structure_preserving_signature;
pub mod withdrawal;

pub struct PartialWallet {
    sig: Signature,
    v: Scalar,
    idx: Option<SignerIndex>,
}
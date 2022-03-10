// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryInto;

use bls12_381::Scalar;

pub use crate::traits::Bytable;
pub use elgamal::elgamal_keygen;
pub use elgamal::ElGamalKeyPair;
pub use elgamal::PublicKey;
pub use error::CoconutError;
pub use scheme::aggregation::aggregate_signature_shares;
pub use scheme::aggregation::aggregate_verification_keys;
pub use scheme::issuance::blind_sign;
pub use scheme::issuance::prepare_blind_sign;
pub use scheme::issuance::BlindSignRequest;
pub use scheme::keygen::ttp_keygen;
pub use scheme::keygen::KeyPair;
pub use scheme::keygen::VerificationKey;
pub use scheme::setup::setup;
pub use scheme::setup::Parameters;
pub use scheme::verification::prove_bandwidth_credential;
pub use scheme::verification::verify_credential;
pub use scheme::verification::Theta;
pub use scheme::BlindedSignature;
pub use scheme::Signature;
pub use scheme::SignatureShare;
pub use traits::Base58;
pub use utils::hash_to_scalar;

pub mod elgamal;
mod error;
mod impls;
mod proofs;
mod scheme;
#[cfg(test)]
mod tests;
mod traits;
mod utils;

pub type Attribute = Scalar;
pub type PrivateAttribute = Attribute;
pub type PublicAttribute = Attribute;

impl Bytable for Attribute {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CoconutError> {
        Ok(Attribute::from_bytes(slice.try_into().unwrap()).unwrap())
    }
}

impl Base58 for Attribute {}

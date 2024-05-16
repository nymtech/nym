// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use bls12_381::Scalar;
use std::convert::TryInto;

pub use bls12_381::G1Projective;
pub use scheme::aggregation::aggregate_verification_keys;
pub use scheme::aggregation::aggregate_wallets;
pub use scheme::identify;
pub use scheme::keygen::ttp_keygen;
pub use scheme::keygen::{generate_keypair_user, generate_keypair_user_from_seed};
pub use scheme::keygen::{KeyPairAuth, PublicKeyUser, SecretKeyUser, VerificationKeyAuth};
pub use scheme::setup;
pub use scheme::withdrawal::issue;
pub use scheme::withdrawal::issue_verify;
pub use scheme::withdrawal::withdrawal_request;
pub use scheme::withdrawal::WithdrawalRequest;
pub use scheme::PartialWallet;
pub use scheme::PayInfo;
pub use setup::GroupParameters;
pub use traits::Base58;

pub use crate::error::CompactEcashError;
pub use crate::traits::Bytable;

pub mod constants;
pub mod error;
mod impls;
mod proofs;
pub mod scheme;
pub mod tests;
mod traits;
pub mod utils;

pub type Attribute = Scalar;

impl Bytable for Attribute {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CompactEcashError> {
        let maybe_attribute = Attribute::from_bytes(
            slice
                .try_into()
                .map_err(|_| CompactEcashError::ScalarDeserializationFailure)?,
        );
        if maybe_attribute.is_none().into() {
            Err(CompactEcashError::ScalarDeserializationFailure)
        } else {
            // safety: this unwrap is fine as we've just checked the element is not none
            #[allow(clippy::unwrap_used)]
            Ok(maybe_attribute.unwrap())
        }
    }
}

// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

use bls12_381::Scalar;
use std::sync::OnceLock;

pub use crate::error::CompactEcashError;
pub use crate::traits::Bytable;
pub use bls12_381::G1Projective;
pub use common_types::{BlindedSignature, Signature};
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

pub mod common_types;
pub mod constants;
pub mod error;
mod helpers;
mod proofs;
pub mod scheme;
pub mod tests;
mod traits;
pub mod utils;

pub type Attribute = Scalar;

pub fn ecash_parameters() -> &'static setup::Parameters {
    static ECACH_PARAMS: OnceLock<setup::Parameters> = OnceLock::new();
    ECACH_PARAMS.get_or_init(|| setup::Parameters::new(constants::NB_TICKETS))
}

pub fn ecash_group_parameters() -> &'static setup::GroupParameters {
    static ECACH_PARAMS: OnceLock<setup::GroupParameters> = OnceLock::new();
    ECACH_PARAMS.get_or_init(|| setup::GroupParameters::new(constants::ATTRIBUTES_LEN))
}

// if anything changes here you MUST correctly increase semver of this library
pub(crate) fn binary_serialiser() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}

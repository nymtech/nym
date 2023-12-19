// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// A `Result` alias where the `Err` case is `coconut_rs::Error`.
pub type Result<T> = std::result::Result<T, CoconutError>;

#[derive(Error, Debug)]
pub enum CoconutError {
    #[error("Setup error: {0}")]
    Setup(String),

    #[error("encountered error during keygen")]
    Keygen,

    #[error("Issuance related error: {0}")]
    Issuance(String),

    #[error("Tried to prepare blind sign request for higher than specified number of attributes (max: {}, requested: {})", max, requested)]
    IssuanceMaxAttributes { max: usize, requested: usize },

    #[error("Interpolation error: {0}")]
    Interpolation(String),

    #[error("Aggregation error: {0}")]
    Aggregation(String),

    #[error("Unblind error: {0}")]
    Unblind(String),

    #[error("Verification error: {0}")]
    Verification(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error(
        "Deserailization error, expected at least {} bytes, got {}",
        min,
        actual
    )]
    DeserializationMinLength { min: usize, actual: usize },

    #[error("Tried to deserialize {object} with bytes of invalid length. Expected {actual} < {object} or {modulus_target} % {modulus} == 0")]
    DeserializationInvalidLength {
        actual: usize,
        target: usize,
        modulus_target: usize,
        modulus: usize,
        object: String,
    },

    #[error("received an array of unexpected size for deserialization of {typ}. got {received} but expected {expected}")]
    UnexpectedArrayLength {
        typ: String,
        received: usize,
        expected: usize,
    },

    #[error("failed to decode the base58 representation: {0}")]
    Base58DecodingFailure(#[from] bs58::decode::Error),

    #[error("failed to deserialize scalar from the received bytes - it might not have been canonically encoded")]
    ScalarDeserializationFailure,

    #[error("failed to deserialize G1Projective point from the received bytes - it might not have been canonically encoded")]
    G1ProjectiveDeserializationFailure,
}

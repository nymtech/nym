// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CompactEcashError>;

#[derive(Error, Debug)]
pub enum CompactEcashError {
    //SW TODO Legacy error to avoid messing up PR stack, remove and adapt once collapsed
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    //SW TODO Legacy error to avoid messing up PR stack, remove and adapt once collapsed
    #[error("Expiration Date related error: {0}")]
    ExpirationDate(String),

    #[error("failed to verify expiration date signatures")]
    ExpirationDateSignatureVerification,

    #[error("failed to validate expiration date signatures")]
    ExpirationDateSignatureValidity,

    #[error("empty set for aggregation")]
    AggregationEmptySet,

    #[error("duplicate indices for aggregation")]
    AggregationDuplicateIndices,

    #[error("aggregation verification error")]
    AggregationVerification,

    #[error("different element size for aggregation")]
    AggregationSizeMismatch,

    #[error("withdrawal request failed to verify")]
    WithdrawalRequestVerification,

    #[error("invalid key generation parameters")]
    KeygenParameters,

    #[error("signing authority's key is too short")]
    KeyTooShort,

    #[error("empty/incomplete set of coordinates for interpolation")]
    InterpolationSetSize,

    #[error("issuance verification failed")]
    IssuanceVerification,

    #[error("trying to spend more than what's available. Spending : {spending}, available : {remaining}")]
    SpendExceedsAllowance { spending: u64, remaining: u64 },

    #[error("signature failed validity check")]
    SpendSignaturesValidity,

    #[error("signature failed verification check")]
    SpendSignaturesVerification,

    #[error("duplicate serial number in the payment")]
    SpendDuplicateSerialNumber,

    #[error("given spend date is too late")]
    SpendDateTooLate,

    #[error("given spend date is too early")]
    SpendDateTooEarly,

    #[error("ZK proof failed to verify")]
    SpendZKProofVerification,

    #[error("could not decode base 58 string - {0}")]
    MalformedString(#[from] bs58::decode::Error),

    #[error("failed to verify coin indices signatures")]
    CoinIndicesSignatureVerification,

    #[error(
        "deserialization error, expected at least {} bytes, got {}",
        min,
        actual
    )]
    DeserializationMinLength { min: usize, actual: usize },

    #[error("tried to deserialize {object} with bytes of invalid length. Expected {actual} < {target} or {modulus_target} % {modulus} == 0")]
    DeserializationInvalidLength {
        actual: usize,
        target: usize,
        modulus_target: usize,
        modulus: usize,
        object: String,
    },

    #[error("failed to deserialize scalar from the received bytes - it might not have been canonically encoded")]
    ScalarDeserializationFailure,

    #[error("failed to deserialize G1Projective point from the received bytes - it might not have been canonically encoded")]
    G1ProjectiveDeserializationFailure,
}

// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DkgError {
    #[error("Provided set of values contained duplicate coordinate")]
    DuplicateCoordinate,

    #[error("The public key is malformed")]
    MalformedPublicKey,

    #[error("The decryption key is malformed")]
    MalformedDecryptionKey,

    #[error("Could not solve the discrete log")]
    UnsolvableDiscreteLog,

    #[error("Received share is malformed")]
    MalformedShare,

    #[error("The share encrypted under index {0} doesn't exist")]
    UnavailableCiphertext(usize),

    #[error("The provided lookup table is mismatched")]
    MismatchedLookupTable,

    #[error("Failed to verify proof of discrete logarithm")]
    InvalidProofOfDiscreteLog,

    #[error("Tried to construct proof of sharing with an invalid instance")]
    MalformedProofOfSharingInstance,

    #[error("Tried to construct proof of chunking with an invalid instance")]
    MalformedProofOfChunkingInstance,

    #[error("Aborted construction of proof of chunking - could not complete it within specified number of attempts")]
    AbortedProofOfChunking,

    #[error("Tried to update the decryption key to an epoch in the past")]
    TargetEpochUpdateInThePast,

    #[error("Provided epoch is malformed")]
    MalformedEpoch,

    #[error("Provided node is not a valid parent")]
    NotAValidParent,

    #[error("Provided decryption key has expired")]
    ExpiredKey,

    #[error("Provided threshold value ({actual}) is either 0 or larger than the total number of the participating parties ({participating})")]
    InvalidThreshold { actual: usize, participating: usize },

    #[error(
    "Provided ciphertext has been generated for a different number of participating parties (expected: {expected}, actual: {actual})"
    )]
    WrongCiphertextSize { actual: usize, expected: usize },

    #[error(
    "Provided public coefficients have been generated for a different number of participating parties (expected: {expected}, actual: {actual})"
    )]
    WrongPublicCoefficientsSize { actual: usize, expected: usize },

    #[error("The provided ciphertexts failed integrity check")]
    FailedCiphertextIntegrityCheck,

    #[error("The provided proof of secret sharing was invalid")]
    InvalidProofOfSharing,

    #[error("The provided proof of chunking was invalid")]
    InvalidProofOfChunking,
}

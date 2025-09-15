// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::ed25519::Ed25519RecoveryError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UpgradeModeCheckError {
    #[error("failed to decode jwt metadata")]
    TokenMetadataDecodeFailure { source: jwt_simple::Error },

    #[error("the jwt metadata didn't contain explicit public key")]
    MissingTokenPublicKey,

    #[error("the attached public key was not valid ed25519 public key")]
    MalformedEd25519PublicKey { source: Ed25519RecoveryError },

    #[error("failed to verify the jwt: {source}")]
    JwtVerificationFailure { source: jwt_simple::Error },

    #[error("failed to retrieve attestation from {url}:{source}")]
    AttestationRetrievalFailure { url: String, source: reqwest::Error },
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "coconut")]
use coconut_interface::CoconutError;
use crypto::asymmetric::encryption::KeyRecoveryError;
use validator_client::ValidatorClientError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("The detailed description is yet to be determined")]
    BandwidthCredentialError,

    #[error("Could not contact any validator")]
    NoValidatorsAvailable,

    #[cfg(feature = "coconut")]
    #[error("Run into a coconut error - {0}")]
    CoconutError(#[from] CoconutError),

    #[error("Run into a validato client error - {0}")]
    ValidatorClientError(#[from] ValidatorClientError),

    #[error("Not enough public attributes were specified")]
    NotEnoughPublicAttributes,

    #[error("Bandwidth is expected to be represented on 8 bytes")]
    InvalidBandwidthSize,

    #[error("Bandwidth operation overflowed. {0}")]
    BandwidthOverflow(String),

    #[error("There is not associated bandwidth for the given client")]
    MissingBandwidth,

    #[error("Could not parse the key - {0}")]
    ParsePublicKey(#[from] KeyRecoveryError),
}

// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

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

    #[error("Ran into a coconut error - {0}")]
    CoconutError(#[from] CoconutError),

    #[error("Ran into a validator client error - {0}")]
    ValidatorClientError(#[from] ValidatorClientError),

    #[error("Bandwidth operation overflowed. {0}")]
    BandwidthOverflow(String),

    #[error("There is not associated bandwidth for the given client")]
    MissingBandwidth,

    #[error("Could not parse the key - {0}")]
    ParsePublicKey(#[from] KeyRecoveryError),

    #[error("Could not gather enough signature shares")]
    NotEnoughShares,
}

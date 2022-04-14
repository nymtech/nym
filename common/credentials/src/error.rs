// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "coconut")]
use coconut_interface::{error::CoconutInterfaceError, CoconutError};
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
    #[error("Ran into a coconut error - {0}")]
    CoconutError(#[from] CoconutError),

    #[cfg(feature = "coconut")]
    #[error("Ran into a coconut interface error - {0}")]
    CoconutInterfaceError(#[from] CoconutInterfaceError),

    #[error("Ran into a validator client error - {0}")]
    ValidatorClientError(#[from] ValidatorClientError),

    #[error("Bandwidth operation overflowed. {0}")]
    BandwidthOverflow(String),

    #[error("There is not associated bandwidth for the given client")]
    MissingBandwidth,

    #[error("Could not parse the key - {0}")]
    ParsePublicKey(#[from] KeyRecoveryError),
}

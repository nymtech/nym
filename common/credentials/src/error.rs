// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::CoconutError;
use nym_crypto::asymmetric::encryption::KeyRecoveryError;
use nym_validator_client::ValidatorClientError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error")]
    IOError(#[from] std::io::Error),

    #[error("failed to (de)serialize credential structure: {0}")]
    SerializationFailure(#[from] bincode::Error),

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

    #[error("Could not gather enough signature shares. Try again using the recovery command")]
    NotEnoughShares,

    #[error("Could not aggregate signature shares - {0}. Try again using the recovery command")]
    SignatureAggregationError(CoconutError),

    #[error("Could not deserialize bandwidth voucher - {0}")]
    BandwidthVoucherDeserializationError(String),
}
